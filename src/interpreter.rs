use crate::ast::*;
use crate::error::RuntimeError;
use crate::gc::{GcHeap, Remap};
use crate::scope::{ScopeContext, ScopeManager};
use crate::value::*;
use std::cell::RefCell;
use std::rc::Rc;

/// The main Shæll interpreter — walks the AST recursively.
/// Owns a GC heap for all value allocation.
pub struct Interpreter {
    pub heap: GcHeap,
    pub scope_manager: ScopeManager,
    pub global_scope: Rc<RefCell<ScopeContext>>,
    pub return_value: Option<ValueRef>,
    pub should_return: bool,
    pub should_break: bool,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    pub fn new() -> Self {
        let global_scope = Rc::new(RefCell::new(ScopeContext::new()));
        let mut scope_manager = ScopeManager::new();
        scope_manager.push_scope(global_scope.clone());
        let this = Interpreter {
            heap: GcHeap::new(),
            scope_manager,
            global_scope,
            return_value: None,
            should_return: false,
            should_break: false,
        };
        debug_assert!(
            !this.should_return,
            "new: should_return must be false initially"
        );
        debug_assert!(
            !this.should_break,
            "new: should_break must be false initially"
        );
        debug_assert!(
            this.return_value.is_none(),
            "new: return_value must be None initially"
        );
        this
    }

    /// After a GC compaction, remap all GcRefs in the scope stack
    /// and return_value so they point to the new object locations.
    fn fix_gc_roots(&mut self) {
        let map = self.heap.last_forward_map.borrow();
        if map.is_empty() {
            return;
        }
        self.global_scope.borrow_mut().remap(&map);
        self.scope_manager.remap(&map);
        if let Some(ref mut rv) = self.return_value {
            for &(old, new) in map.iter() {
                if old == rv.offset() {
                    *rv = ValueRef::from_offset(new);
                    break;
                }
            }
        }
        drop(map);
        self.heap.clear_forward_map();
        debug_assert!(
            self.heap.last_forward_map.borrow().is_empty(),
            "fix_gc_roots: forward map not cleared"
        );
    }

    pub fn execute_program(&mut self, program: &Program) -> Result<ValueRef, RuntimeError> {
        if let Some(args_def) = &program.args {
            self.handle_program_args(args_def)?;
        }
        let result = self.execute_statements(&program.statements);
        if let Ok(ref val) = result {
            debug_assert!(
                self.heap.is_live_by_offset(val.offset()),
                "execute_program: result value is not live"
            );
        }
        result
    }

    fn handle_program_args(&mut self, args_def: &ProgramArgs) -> Result<(), RuntimeError> {
        let table = self.heap.allocate(Value::table());
        self.scope_manager
            .new_top_level_value(&args_def.table_name, table)?;
        debug_assert!(
            self.scope_manager.get_value(&args_def.table_name).is_some(),
            "handle_program_args: '{}' not in scope after insertion",
            args_def.table_name
        );
        Ok(())
    }

    // ============================================================
    // Statement Execution
    // ============================================================

    pub fn execute_statements(&mut self, stmts: &[Stmt]) -> Result<ValueRef, RuntimeError> {
        let mut last_value: ValueRef = self.heap.allocate(Value::Null);
        self.fix_gc_roots();
        for stmt in stmts {
            if self.should_return || self.should_break {
                break;
            }
            last_value = self.execute_statement(stmt)?;
            self.fix_gc_roots();
        }
        debug_assert!(
            self.heap.is_live_by_offset(last_value.offset()),
            "execute_statements: last_value is not live"
        );
        Ok(last_value)
    }

    fn execute_statement(&mut self, stmt: &Stmt) -> Result<ValueRef, RuntimeError> {
        let result = match stmt {
            Stmt::Expr(expr) => self.eval_expression(expr),
            Stmt::If(if_stmt) => self.execute_if(if_stmt),
            Stmt::For(for_loop) => self.execute_for(for_loop),
            Stmt::While(while_loop) => self.execute_while(while_loop),
            Stmt::Foreach(foreach) => self.execute_foreach(foreach),
            Stmt::ForeachKeyValue(foreach_kv) => self.execute_foreach_kv(foreach_kv),
            Stmt::Return(expr) => {
                let value = self.eval_expression(expr)?;
                self.return_value = Some(value);
                self.should_return = true;
                Ok(value)
            }
            Stmt::Throw(expr) => {
                let value = self.eval_expression(expr)?;
                let msg = self
                    .heap
                    .with_ref(value, |v| v.to_sstring())
                    .unwrap_or_else(|_| "error".to_string());
                Err(RuntimeError::new(msg))
            }
            Stmt::Break => {
                self.should_break = true;
                Ok(self.heap.allocate(Value::Null))
            }
            Stmt::FunctionDef(func_def) => self.execute_function_def(func_def),
            Stmt::PipeProgram(pipe) => self.execute_pipe_program_stmt(pipe),
        };
        // Post-condition: no invalid state after statement execution
        debug_assert!(
            !(self.should_return && self.should_break),
            "execute_statement: cannot both return and break"
        );
        debug_assert!(
            !self.should_return || self.return_value.is_some(),
            "execute_statement: should_return set but return_value is None"
        );
        result
    }

    // ============================================================
    // Control Flow
    // ============================================================

    fn execute_if(&mut self, if_stmt: &IfStmt) -> Result<ValueRef, RuntimeError> {
        let condition = self.eval_expression(&if_stmt.condition)?;
        let cond_bool = self
            .heap
            .with_ref(condition, |v| v.to_bool().unwrap_or(false));

        let result = if cond_bool {
            self.scope_manager.push_new_scope();
            let r = self.execute_statements(&if_stmt.then_body);
            self.scope_manager.pop_scope();
            r
        } else if let Some(else_body) = &if_stmt.else_body {
            self.scope_manager.push_new_scope();
            let r = self.execute_statements(else_body);
            self.scope_manager.pop_scope();
            r
        } else {
            Ok(self.heap.allocate(Value::Null))
        };
        if let Ok(ref val) = result {
            debug_assert!(
                self.heap.is_live_by_offset(val.offset()),
                "execute_if: result value is not live"
            );
        }
        result
    }

    fn execute_while(&mut self, while_loop: &WhileLoop) -> Result<ValueRef, RuntimeError> {
        let mut last_value = self.heap.allocate(Value::Null);
        loop {
            let condition = self.eval_expression(&while_loop.condition)?;
            let cond_bool = self
                .heap
                .with_ref(condition, |v| v.to_bool().unwrap_or(false));
            if !cond_bool {
                break;
            }
            self.scope_manager.push_new_scope();
            last_value = self.execute_statements(&while_loop.body)?;
            self.scope_manager.pop_scope();
            if self.should_return {
                break;
            }
            if self.should_break {
                self.should_break = false;
                break;
            }
        }
        debug_assert!(
            self.heap.is_live_by_offset(last_value.offset()),
            "execute_while: last_value is not live"
        );
        Ok(last_value)
    }

    fn execute_for(&mut self, for_loop: &ForLoop) -> Result<ValueRef, RuntimeError> {
        self.eval_expression(&for_loop.init)?;
        let mut last_value = self.heap.allocate(Value::Null);
        loop {
            let condition = self.eval_expression(&for_loop.condition)?;
            let cond_bool = self
                .heap
                .with_ref(condition, |v| v.to_bool().unwrap_or(false));
            if !cond_bool {
                break;
            }
            self.scope_manager.push_new_scope();
            last_value = self.execute_statements(&for_loop.body)?;
            self.scope_manager.pop_scope();
            if self.should_return {
                break;
            }
            if self.should_break {
                self.should_break = false;
                break;
            }
            self.eval_expression(&for_loop.update)?;
        }
        debug_assert!(
            self.heap.is_live_by_offset(last_value.offset()),
            "execute_for: last_value is not live"
        );
        Ok(last_value)
    }

    fn execute_foreach(&mut self, foreach: &ForeachLoop) -> Result<ValueRef, RuntimeError> {
        let collection = self.eval_expression(&foreach.collection)?;
        let keys = self.get_keys_for_collection(collection)?;

        let mut last_value = self.heap.allocate(Value::Null);
        for key in &keys {
            self.scope_manager.push_new_scope();

            let value = {
                let coll_val = self.heap.with_ref(collection, |v| v.clone());
                let key_val = self.heap.with_ref(*key, |v| v.clone());
                coll_val.index(&key_val, &self.heap)?
            };

            self.scope_manager
                .new_top_level_value(&foreach.var, value)?;
            last_value = self.execute_statements(&foreach.body)?;
            self.scope_manager.pop_scope();

            if self.should_return {
                break;
            }
            if self.should_break {
                self.should_break = false;
                break;
            }
        }
        debug_assert!(
            self.heap.is_live_by_offset(last_value.offset()),
            "execute_foreach: last_value is not live"
        );
        Ok(last_value)
    }

    fn execute_foreach_kv(
        &mut self,
        foreach: &ForeachKeyValueLoop,
    ) -> Result<ValueRef, RuntimeError> {
        let collection = self.eval_expression(&foreach.collection)?;
        let keys = self.get_keys_for_collection(collection)?;

        let mut last_value = self.heap.allocate(Value::Null);
        for key in keys {
            self.scope_manager.push_new_scope();

            let value = {
                let coll_val = self.heap.with_ref(collection, |v| v.clone());
                let key_val = self.heap.with_ref(key, |v| v.clone());
                coll_val.index(&key_val, &self.heap)?
            };

            self.scope_manager
                .new_top_level_value(&foreach.key_var, key)?;
            self.scope_manager
                .new_top_level_value(&foreach.value_var, value)?;
            last_value = self.execute_statements(&foreach.body)?;
            self.scope_manager.pop_scope();

            if self.should_return {
                break;
            }
            if self.should_break {
                self.should_break = false;
                break;
            }
        }
        debug_assert!(
            self.heap.is_live_by_offset(last_value.offset()),
            "execute_foreach_kv: last_value is not live"
        );
        Ok(last_value)
    }

    /// Extract keys from a collection for iteration.
    fn get_keys_for_collection(&self, collection: ValueRef) -> Result<Vec<ValueRef>, RuntimeError> {
        debug_assert!(
            self.heap.is_live_by_offset(collection.offset()),
            "get_keys_for_collection: collection is not live"
        );
        let coll_val = self.heap.with_ref(collection, |v| v.clone());
        coll_val.get_keys(&self.heap)
    }

    // ============================================================
    // Functions
    // ============================================================

    fn execute_function_def(&mut self, func_def: &FunctionDef) -> Result<ValueRef, RuntimeError> {
        let func = FuncData {
            name: func_def.name.clone(),
            params: func_def.params.clone(),
            body: func_def.body.clone(),
            captured_scope: self.scope_manager.copy_scopes(),
        };
        let func_ref = self.heap.allocate(Value::Function(func));
        self.scope_manager
            .new_top_level_value(&func_def.name, func_ref)?;
        debug_assert!(
            self.scope_manager.get_value(&func_def.name).is_some(),
            "execute_function_def: function '{}' not in scope after definition",
            func_def.name
        );
        Ok(self.heap.allocate(Value::Null))
    }

    // ============================================================
    // Expression Evaluation
    // ============================================================

    pub fn eval_expression(&mut self, expr: &Expr) -> Result<ValueRef, RuntimeError> {
        let result = match expr {
            Expr::Number(n) => Ok(self.heap.allocate(Value::Number(*n))),
            Expr::String_(s) => self.eval_string_literal(s),
            Expr::Boolean(b) => Ok(self.heap.allocate(Value::Bool(*b))),
            Expr::Null => Ok(self.heap.allocate(Value::Null)),

            Expr::Identifier(name) => self.eval_identifier(name),

            Expr::UnaryNeg(e) => {
                let val = self.eval_expression(e)?;
                let n = self.heap.with_ref(val, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(-n)))
            }
            Expr::UnaryPlus(e) => {
                let val = self.eval_expression(e)?;
                let n = self.heap.with_ref(val, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(n)))
            }
            Expr::UnaryNot(e) => {
                let val = self.eval_expression(e)?;
                let b = self.heap.with_ref(val, |v| v.to_bool().unwrap_or(false));
                Ok(self.heap.allocate(Value::Bool(!b)))
            }
            Expr::Deref(e) => {
                let val = self.eval_expression(e)?;
                let path = self.heap.with_ref(val, |v| v.to_sstring())?;
                Ok(self.heap.allocate(Value::File(path)))
            }

            Expr::Add(l, r) => self.eval_add(l, r),
            Expr::Sub(l, r) => self.eval_arithmetic(l, r, |a, b| a - b),
            Expr::Mult(l, r) => self.eval_mult(l, r),
            Expr::Div(l, r) => self.eval_arithmetic(l, r, |a, b| a / b),
            Expr::Mod(l, r) => self.eval_arithmetic(l, r, |a, b| a % b),
            Expr::Pow(l, r) => self.eval_arithmetic(l, r, |a, b| a.powf(b)),

            Expr::LT(l, r) => self.eval_comparison(l, r, |a, b| a < b),
            Expr::LEQ(l, r) => self.eval_comparison(l, r, |a, b| a <= b),
            Expr::GT(l, r) => self.eval_comparison(l, r, |a, b| a > b),
            Expr::GEQ(l, r) => self.eval_comparison(l, r, |a, b| a >= b),
            Expr::EQ(l, r) => self.eval_equality(l, r),
            Expr::NEQ(l, r) => {
                let eq_val = self.eval_equality(l, r)?;
                let b = self.heap.with_ref(eq_val, |v| v.to_bool().unwrap_or(false));
                Ok(self.heap.allocate(Value::Bool(!b)))
            }
            Expr::LAnd(l, r) => self.eval_logical_and(l, r),
            Expr::LOr(l, r) => self.eval_logical_or(l, r),

            Expr::Assign(l, r) => self.eval_assign(l, r),
            Expr::PlusEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Add),
            Expr::MinusEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Sub),
            Expr::MultEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Mult),
            Expr::DivEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Div),
            Expr::ModEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Mod),
            Expr::PowEq(l, r) => self.eval_compound_assign(l, r, CompoundOp::Pow),

            Expr::Let(name) => {
                self.scope_manager
                    .new_top_level_value(name, self.heap.allocate(Value::Null))?;
                Ok(self.heap.allocate(Value::Null))
            }
            Expr::LetAssign(name, e) => {
                let value = self.eval_expression(e)?;
                self.scope_manager.new_top_level_value(name, value)?;
                Ok(self.heap.allocate(Value::Null))
            }

            Expr::IndexColon(obj_expr, key) => {
                let obj = self.eval_expression(obj_expr)?;
                let key_val = Value::String(key.clone());
                let obj_val = self.heap.with_ref(obj, |v| v.clone());
                obj_val.index(&key_val, &self.heap)
            }
            Expr::IndexSubscript(obj_expr, idx_expr) => {
                let obj = self.eval_expression(obj_expr)?;
                let idx = self.eval_expression(idx_expr)?;
                let obj_val = self.heap.with_ref(obj, |v| v.clone());
                let idx_val = self.heap.with_ref(idx, |v| v.clone());
                obj_val.index(&idx_val, &self.heap)
            }
            Expr::Call(func_expr, args) => {
                let func_val = self.eval_expression(func_expr)?;
                let mut eval_args = Vec::new();
                for arg in args {
                    eval_args.push(self.eval_expression(arg)?);
                }
                self.call_value(func_val, &eval_args)
            }

            Expr::Object(fields) => {
                let table = self.heap.allocate(Value::table());
                for (field, value_expr) in fields {
                    let value = self.eval_expression(value_expr)?;
                    match field {
                        ObjectField::Identifier(name) => {
                            let key_val = Value::String(name.clone());
                            self.heap
                                .with_mut(table, |v| v.set_index(&key_val, value))?;
                        }
                        ObjectField::Subscript(index_expr) => {
                            let idx = self.eval_expression(index_expr)?;
                            let idx_val = self.heap.with_ref(idx, |v| v.clone());
                            self.heap
                                .with_mut(table, |v| v.set_index(&idx_val, value))?;
                        }
                    }
                }
                Ok(table)
            }

            Expr::AnonFnDef(anon) => {
                let func = FuncData {
                    name: "<anonymous>".to_string(),
                    params: anon.params.clone(),
                    body: anon.body.clone(),
                    captured_scope: self.scope_manager.copy_scopes(),
                };
                Ok(self.heap.allocate(Value::Function(func)))
            }

            Expr::Try(stmts) => self.execute_try(stmts),

            Expr::ProgProgram(prog) => self.execute_prog_program(prog),
        };
        if let Ok(ref val) = result {
            debug_assert!(
                self.heap.is_live_by_offset(val.offset()),
                "eval_expression: returned value is not live"
            );
        }
        result
    }

    // ============================================================
    // String Evaluation
    // ============================================================

    fn eval_string_literal(&mut self, s: &StringLit) -> Result<ValueRef, RuntimeError> {
        let mut result = String::new();
        for part in &s.parts {
            match part {
                StringPart::Text(t) => result.push_str(t),
                StringPart::Newline => result.push('\n'),
                StringPart::EscapedDollar => result.push('$'),
                StringPart::EscapedBackslash => result.push('\\'),
                StringPart::Interpolation(expr) => {
                    let val = self.eval_expression(expr)?;
                    let s = self
                        .heap
                        .with_ref(val, |v| v.to_sstring().unwrap_or_default());
                    result.push_str(&s);
                }
            }
        }
        let allocated = self.heap.allocate(Value::String(result));
        debug_assert!(
            self.heap.is_live_by_offset(allocated.offset()),
            "eval_string_literal: result is not live"
        );
        Ok(allocated)
    }

    // ============================================================
    // Identifier
    // ============================================================

    fn eval_identifier(&self, name: &str) -> Result<ValueRef, RuntimeError> {
        debug_assert!(!name.is_empty(), "eval_identifier: name is empty");
        if let Some(val) = self.scope_manager.get_value(name) {
            return Ok(val);
        }
        // Shæll: undeclared identifiers evaluate to a File
        Ok(self.heap.allocate(Value::File(name.to_string())))
    }

    // ============================================================
    // Binary Operations
    // ============================================================

    fn eval_add(&mut self, l: &Expr, r: &Expr) -> Result<ValueRef, RuntimeError> {
        let left = self.eval_expression(l)?;
        let right = self.eval_expression(r)?;

        let lt = self.heap.with_ref(left, |v| v.get_type_name().to_string());
        let rt = self.heap.with_ref(right, |v| v.get_type_name().to_string());

        debug_assert!(
            matches!(
                lt.as_str(),
                "number" | "string" | "bstring" | "bool" | "null" | "file" | "table" | "function"
            ),
            "eval_add: left operand has unknown type '{}'",
            lt
        );
        debug_assert!(
            matches!(
                rt.as_str(),
                "number" | "string" | "bstring" | "bool" | "null" | "file" | "table" | "function"
            ),
            "eval_add: right operand has unknown type '{}'",
            rt
        );

        // String concatenation
        if lt == "string" || lt == "bstring" || rt == "string" || rt == "bstring" {
            let ls = self
                .heap
                .with_ref(left, |v| v.to_sstring().unwrap_or_default());
            let rs = self
                .heap
                .with_ref(right, |v| v.to_sstring().unwrap_or_default());
            return Ok(self.heap.allocate(Value::String(ls + &rs)));
        }

        // Numeric addition
        let ln = self.heap.with_ref(left, |v| v.to_number())?;
        let rn = self.heap.with_ref(right, |v| v.to_number())?;
        Ok(self.heap.allocate(Value::Number(ln + rn)))
    }

    fn eval_mult(&mut self, l: &Expr, r: &Expr) -> Result<ValueRef, RuntimeError> {
        let left = self.eval_expression(l)?;
        let right = self.eval_expression(r)?;

        let lt = self.heap.with_ref(left, |v| v.get_type_name().to_string());
        let rt = self.heap.with_ref(right, |v| v.get_type_name().to_string());

        debug_assert!(
            matches!(
                lt.as_str(),
                "number" | "string" | "bstring" | "bool" | "null" | "file" | "table" | "function"
            ),
            "eval_mult: left operand has unknown type '{}'",
            lt
        );
        debug_assert!(
            matches!(
                rt.as_str(),
                "number" | "string" | "bstring" | "bool" | "null" | "file" | "table" | "function"
            ),
            "eval_mult: right operand has unknown type '{}'",
            rt
        );

        if lt == "string" && rt == "number" {
            let s = self
                .heap
                .with_ref(left, |v| v.to_sstring().unwrap_or_default());
            let n = self.heap.with_ref(right, |v| v.to_number())? as usize;
            return Ok(self.heap.allocate(Value::String(s.repeat(n))));
        }
        if lt == "number" && rt == "string" {
            let n = self.heap.with_ref(left, |v| v.to_number())? as usize;
            let s = self
                .heap
                .with_ref(right, |v| v.to_sstring().unwrap_or_default());
            return Ok(self.heap.allocate(Value::String(s.repeat(n))));
        }

        let ln = self.heap.with_ref(left, |v| v.to_number())?;
        let rn = self.heap.with_ref(right, |v| v.to_number())?;
        Ok(self.heap.allocate(Value::Number(ln * rn)))
    }

    fn eval_arithmetic<F>(&mut self, l: &Expr, r: &Expr, op: F) -> Result<ValueRef, RuntimeError>
    where
        F: Fn(f64, f64) -> f64,
    {
        let left = self.eval_expression(l)?;
        let right = self.eval_expression(r)?;
        debug_assert!(
            self.heap.with_ref(left, |v| v.to_number().is_ok()),
            "eval_arithmetic: left operand is not a number"
        );
        debug_assert!(
            self.heap.with_ref(right, |v| v.to_number().is_ok()),
            "eval_arithmetic: right operand is not a number"
        );
        let ln = self.heap.with_ref(left, |v| v.to_number())?;
        let rn = self.heap.with_ref(right, |v| v.to_number())?;
        Ok(self.heap.allocate(Value::Number(op(ln, rn))))
    }

    fn eval_comparison<F>(&mut self, l: &Expr, r: &Expr, op: F) -> Result<ValueRef, RuntimeError>
    where
        F: Fn(f64, f64) -> bool,
    {
        let left = self.eval_expression(l)?;
        let right = self.eval_expression(r)?;
        debug_assert!(
            self.heap.is_live_by_offset(left.offset()),
            "eval_comparison: left operand is not live"
        );
        debug_assert!(
            self.heap.is_live_by_offset(right.offset()),
            "eval_comparison: right operand is not live"
        );
        if let (Ok(ln), Ok(rn)) = (
            self.heap.with_ref(left, |v| v.to_number()),
            self.heap.with_ref(right, |v| v.to_number()),
        ) {
            return Ok(self.heap.allocate(Value::Bool(op(ln, rn))));
        }
        let ls = self
            .heap
            .with_ref(left, |v| v.to_sstring().unwrap_or_default());
        let rs = self
            .heap
            .with_ref(right, |v| v.to_sstring().unwrap_or_default());
        Ok(self
            .heap
            .allocate(Value::Bool(op(ls.len() as f64, rs.len() as f64))))
    }

    fn eval_equality(&mut self, l: &Expr, r: &Expr) -> Result<ValueRef, RuntimeError> {
        let left = self.eval_expression(l)?;
        let right = self.eval_expression(r)?;
        debug_assert!(
            self.heap.is_live_by_offset(left.offset()),
            "eval_equality: left operand is not live"
        );
        debug_assert!(
            self.heap.is_live_by_offset(right.offset()),
            "eval_equality: right operand is not live"
        );
        let lv = self.heap.with_ref(left, |v| v.clone());
        let rv = self.heap.with_ref(right, |v| v.clone());
        let is_equal = lv.is_equal(&rv)?;
        Ok(self.heap.allocate(Value::Bool(is_equal)))
    }

    fn eval_logical_and(&mut self, l: &Expr, r: &Expr) -> Result<ValueRef, RuntimeError> {
        let left = self.eval_expression(l)?;
        let lb = self.heap.with_ref(left, |v| v.to_bool().unwrap_or(false));
        if !lb {
            let result = self.heap.allocate(Value::Bool(false));
            debug_assert!(
                self.heap.is_live_by_offset(result.offset()),
                "eval_logical_and: result is not live"
            );
            return Ok(result);
        }
        let right = self.eval_expression(r)?;
        let rb = self.heap.with_ref(right, |v| v.to_bool().unwrap_or(false));
        let result = self.heap.allocate(Value::Bool(rb));
        debug_assert!(
            self.heap.is_live_by_offset(result.offset()),
            "eval_logical_and: result is not live"
        );
        Ok(result)
    }

    fn eval_logical_or(&mut self, l: &Expr, r: &Expr) -> Result<ValueRef, RuntimeError> {
        let left = self.eval_expression(l)?;
        let lb = self.heap.with_ref(left, |v| v.to_bool().unwrap_or(false));
        if lb {
            let result = self.heap.allocate(Value::Bool(true));
            debug_assert!(
                self.heap.is_live_by_offset(result.offset()),
                "eval_logical_or: result is not live"
            );
            return Ok(result);
        }
        let right = self.eval_expression(r)?;
        let rb = self.heap.with_ref(right, |v| v.to_bool().unwrap_or(false));
        let result = self.heap.allocate(Value::Bool(rb));
        debug_assert!(
            self.heap.is_live_by_offset(result.offset()),
            "eval_logical_or: result is not live"
        );
        Ok(result)
    }

    // ============================================================
    // Assignment
    // ============================================================

    fn eval_assign(&mut self, target: &Expr, value: &Expr) -> Result<ValueRef, RuntimeError> {
        let val = self.eval_expression(value)?;
        debug_assert!(
            self.heap.is_live_by_offset(val.offset()),
            "eval_assign: value is not live"
        );
        match target {
            Expr::Identifier(name) => {
                if self.scope_manager.get_value(name).is_some() {
                    self.scope_manager.set_value(name, val)?;
                } else {
                    self.scope_manager.new_top_level_value(name, val)?;
                }
            }
            Expr::IndexColon(obj_expr, key) => {
                let obj = self.eval_expression(obj_expr)?;
                let key_val = Value::String(key.clone());
                self.heap.with_mut(obj, |v| v.set_index(&key_val, val))?;
            }
            Expr::IndexSubscript(obj_expr, idx_expr) => {
                let obj = self.eval_expression(obj_expr)?;
                let idx = self.eval_expression(idx_expr)?;
                let idx_val = self.heap.with_ref(idx, |v| v.clone());
                self.heap.with_mut(obj, |v| v.set_index(&idx_val, val))?;
            }
            _ => {
                return Err(RuntimeError::new("Invalid assignment target".to_string()));
            }
        }
        Ok(val)
    }

    fn eval_compound_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
        op: CompoundOp,
    ) -> Result<ValueRef, RuntimeError> {
        debug_assert!(
            !matches!(target, Expr::Null),
            "eval_compound_assign: target is Null"
        );
        let current = match target {
            Expr::Identifier(name) => self.eval_identifier(name)?,
            Expr::IndexColon(obj_expr, key) => {
                let obj = self.eval_expression(obj_expr)?;
                let obj_val = self.heap.with_ref(obj, |v| v.clone());
                let key_val = Value::String(key.clone());
                obj_val.index(&key_val, &self.heap)?
            }
            Expr::IndexSubscript(obj_expr, idx_expr) => {
                let obj = self.eval_expression(obj_expr)?;
                let idx = self.eval_expression(idx_expr)?;
                let obj_val = self.heap.with_ref(obj, |v| v.clone());
                let idx_val = self.heap.with_ref(idx, |v| v.clone());
                obj_val.index(&idx_val, &self.heap)?
            }
            _ => {
                return Err(RuntimeError::new(
                    "Invalid compound assignment target".to_string(),
                ))
            }
        };

        let new_val_reference = self.eval_expression(value)?;

        let result = self.apply_compound_op(current, new_val_reference, op)?;

        match target {
            Expr::Identifier(name) => {
                self.scope_manager.set_value(name, result)?;
            }
            Expr::IndexColon(obj_expr, key) => {
                let obj = self.eval_expression(obj_expr)?;
                let key_val = Value::String(key.clone());
                self.heap.with_mut(obj, |v| v.set_index(&key_val, result))?;
            }
            Expr::IndexSubscript(obj_expr, idx_expr) => {
                let obj = self.eval_expression(obj_expr)?;
                let idx = self.eval_expression(idx_expr)?;
                let idx_val = self.heap.with_ref(idx, |v| v.clone());
                self.heap.with_mut(obj, |v| v.set_index(&idx_val, result))?;
            }
            _ => {}
        }
        debug_assert!(
            self.heap.is_live_by_offset(result.offset()),
            "eval_compound_assign: result is not live"
        );
        Ok(result)
    }

    /// Apply a compound operation to two ValueRefs, allocating the result on the heap.
    fn apply_compound_op(
        &self,
        left: ValueRef,
        right: ValueRef,
        op: CompoundOp,
    ) -> Result<ValueRef, RuntimeError> {
        debug_assert!(
            self.heap.is_live_by_offset(left.offset()),
            "apply_compound_op: left operand is not live"
        );
        debug_assert!(
            self.heap.is_live_by_offset(right.offset()),
            "apply_compound_op: right operand is not live"
        );
        match op {
            CompoundOp::Add => {
                let lt = self.heap.with_ref(left, |v| v.get_type_name().to_string());
                let rt = self.heap.with_ref(right, |v| v.get_type_name().to_string());
                if lt == "string" || lt == "bstring" || rt == "string" || rt == "bstring" {
                    let ls = self
                        .heap
                        .with_ref(left, |v| v.to_sstring().unwrap_or_default());
                    let rs = self
                        .heap
                        .with_ref(right, |v| v.to_sstring().unwrap_or_default());
                    Ok(self.heap.allocate(Value::String(ls + &rs)))
                } else {
                    let an = self.heap.with_ref(left, |v| v.to_number())?;
                    let bn = self.heap.with_ref(right, |v| v.to_number())?;
                    Ok(self.heap.allocate(Value::Number(an + bn)))
                }
            }
            CompoundOp::Sub => {
                let an = self.heap.with_ref(left, |v| v.to_number())?;
                let bn = self.heap.with_ref(right, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(an - bn)))
            }
            CompoundOp::Mult => {
                let an = self.heap.with_ref(left, |v| v.to_number())?;
                let bn = self.heap.with_ref(right, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(an * bn)))
            }
            CompoundOp::Div => {
                let an = self.heap.with_ref(left, |v| v.to_number())?;
                let bn = self.heap.with_ref(right, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(an / bn)))
            }
            CompoundOp::Mod => {
                let an = self.heap.with_ref(left, |v| v.to_number())?;
                let bn = self.heap.with_ref(right, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(an % bn)))
            }
            CompoundOp::Pow => {
                let an = self.heap.with_ref(left, |v| v.to_number())?;
                let bn = self.heap.with_ref(right, |v| v.to_number())?;
                Ok(self.heap.allocate(Value::Number(an.powf(bn))))
            }
        }
    }

    // ============================================================
    // Try
    // ============================================================

    fn execute_try(&mut self, stmts: &[Stmt]) -> Result<ValueRef, RuntimeError> {
        self.scope_manager.push_new_scope();
        let result = self.execute_statements(stmts);
        self.scope_manager.pop_scope();

        let result_table = self.heap.allocate(Value::table());
        match result {
            Ok(value) => {
                let status_zero = self.heap.allocate(Value::Number(0.0));
                let null_val = self.heap.allocate(Value::Null);
                self.heap.with_mut(result_table, |t| {
                    t.set_index(&Value::String("status".to_string()), status_zero)?;
                    t.set_index(&Value::String("value".to_string()), value)?;
                    t.set_index(&Value::String("error".to_string()), null_val)?;
                    Ok::<(), RuntimeError>(())
                })?;
            }
            Err(err) => {
                let status_one = self.heap.allocate(Value::Number(1.0));
                let null_val = self.heap.allocate(Value::Null);
                let err_val = self.heap.allocate(Value::String(err.message));
                self.heap.with_mut(result_table, |t| {
                    t.set_index(&Value::String("status".to_string()), status_one)?;
                    t.set_index(&Value::String("value".to_string()), null_val)?;
                    t.set_index(&Value::String("error".to_string()), err_val)?;
                    Ok::<(), RuntimeError>(())
                })?;
            }
        }
        debug_assert!(
            self.heap.is_live_by_offset(result_table.offset()),
            "execute_try: result_table is not live"
        );
        Ok(result_table)
    }

    // ============================================================
    // External Program
    // ============================================================

    fn execute_pipe_program_stmt(&mut self, prog: &PipeProgram) -> Result<ValueRef, RuntimeError> {
        use crate::pipe;
        debug_assert!(
            !matches!(*prog.program, Expr::Null),
            "execute_pipe_program_stmt: program expression is Null"
        );
        pipe::execute_pipe_program(&self.heap, prog)
    }

    fn execute_prog_program(&mut self, prog: &ProgProgram) -> Result<ValueRef, RuntimeError> {
        use crate::pipe;
        debug_assert!(
            !matches!(*prog.program, Expr::Null),
            "execute_prog_program: program expression is Null"
        );
        // Evaluate arg expressions to strings
        let mut arg_strings: Vec<String> = Vec::new();
        for arg in &prog.args {
            let val = self.eval_expression(arg)?;
            let s = self
                .heap
                .with_ref(val, |v| v.to_sstring().unwrap_or_default());
            arg_strings.push(s);
        }
        let (return_code, stdout) =
            pipe::execute_prog_program_strs(&self.heap, prog, &arg_strings, None)?;

        if let Some(target_expr) = &prog.pipe_expr {
            let stdout_str = stdout.clone().unwrap_or_default();
            let value = self.heap.allocate(Value::String(stdout_str));
            match target_expr.as_ref() {
                Expr::Identifier(name) => {
                    if self.scope_manager.get_value(name).is_some() {
                        self.scope_manager.set_value(name, value)?;
                    } else {
                        self.scope_manager.new_top_level_value(name, value)?;
                    }
                }
                Expr::Let(name) => {
                    self.scope_manager.new_top_level_value(name, value)?;
                }
                _ => {
                    return Err(RuntimeError::new("Invalid pipe target".to_string()));
                }
            }
        }

        if let Some(target) = &prog.pipe_target {
            match target.as_ref() {
                PipeTarget::ProgProgram(inner_prog) => {
                    let mut inner_args: Vec<String> = Vec::new();
                    for arg in &inner_prog.args {
                        let val = self.eval_expression(arg)?;
                        let s = self
                            .heap
                            .with_ref(val, |v| v.to_sstring().unwrap_or_default());
                        inner_args.push(s);
                    }
                    let (_rc, inner_stdout) = pipe::execute_prog_program_strs(
                        &self.heap,
                        inner_prog,
                        &inner_args,
                        stdout,
                    )?;
                    // Handle the inner program's pipe_expr (e.g. -> let out)
                    if let Some(target_expr) = &inner_prog.pipe_expr {
                        let stdout_str = inner_stdout.unwrap_or_default();
                        let value = self.heap.allocate(Value::String(stdout_str));
                        match target_expr.as_ref() {
                            Expr::Identifier(name) => {
                                if self.scope_manager.get_value(name).is_some() {
                                    self.scope_manager.set_value(name, value)?;
                                } else {
                                    self.scope_manager.new_top_level_value(name, value)?;
                                }
                            }
                            Expr::Let(name) => {
                                self.scope_manager.new_top_level_value(name, value)?;
                            }
                            _ => {
                                return Err(RuntimeError::new("Invalid pipe target".to_string()));
                            }
                        }
                    }
                }
                PipeTarget::PipeProgram(inner_pipe) => {
                    let _ = pipe::execute_pipe_program(&self.heap, inner_pipe)?;
                }
            }
        }

        Ok(self.heap.allocate(Value::Number(return_code as f64)))
    }

    // ============================================================
    // Function Call
    // ============================================================

    fn call_value(
        &mut self,
        func_val: ValueRef,
        args: &[ValueRef],
    ) -> Result<ValueRef, RuntimeError> {
        debug_assert!(!func_val.is_null(), "call_value: func_val GcRef is null");
        debug_assert!(
            !self.heap.with_ref(func_val, |v| matches!(v, Value::Null)),
            "call_value: func_val points to Value::Null"
        );
        let kind = self.get_callable_kind(func_val);
        match kind {
            CallableKind::User(fd) => self.call_user_func(&fd, args),
            CallableKind::Native(nf) => nf(&self.heap, args),
            CallableKind::NotCallable => Err(RuntimeError::new("Not callable".to_string())),
        }
    }

    fn get_callable_kind(&self, func_val: ValueRef) -> CallableKind {
        debug_assert!(!func_val.is_null(), "get_callable_kind: func_val is null");
        self.heap.with_ref(func_val, |v| match v {
            Value::Function(f) => CallableKind::User(f.clone()),
            Value::NativeFunction(n) => CallableKind::Native(n.func.clone()),
            _ => CallableKind::NotCallable,
        })
    }

    fn call_user_func(
        &mut self,
        func: &FuncData,
        args: &[ValueRef],
    ) -> Result<ValueRef, RuntimeError> {
        debug_assert!(!func.name.is_empty(), "call_user_func: func name is empty");
        let saved_scope = self.scope_manager.copy_scopes();
        let old_return_value = self.return_value.take();
        let old_should_return = self.should_return;
        let old_should_break = self.should_break;

        self.should_return = false;
        self.should_break = false;

        self.scope_manager = func.captured_scope.copy_scopes();
        self.scope_manager.push_new_scope();

        for (i, param) in func.params.iter().enumerate() {
            let arg = if i < args.len() {
                args[i]
            } else {
                self.heap.allocate(Value::Null)
            };
            self.scope_manager.new_top_level_value(param, arg)?;
        }

        let result = match &func.body {
            FunctionBody::Block(stmts) => self.execute_statements(stmts),
            FunctionBody::Lambda(expr) => self.eval_expression(expr),
        };

        let final_value = match result {
            Ok(v) => self.return_value.take().unwrap_or(v),
            Err(e) => {
                self.scope_manager = saved_scope;
                self.return_value = old_return_value;
                self.should_return = old_should_return;
                self.should_break = old_should_break;
                return Err(e);
            }
        };

        self.scope_manager = saved_scope;
        self.return_value = old_return_value;
        self.should_return = old_should_return;
        self.should_break = old_should_break;

        debug_assert!(
            self.heap.is_live_by_offset(final_value.offset()),
            "call_user_func: final_value is not live"
        );
        debug_assert!(
            !(self.should_return && self.should_break),
            "call_user_func: both should_return and should_break set after restore"
        );
        Ok(final_value)
    }
}

// ============================================================
// Helper types
// ============================================================

#[derive(Clone)]
enum CallableKind {
    User(FuncData),
    Native(NativeFn),
    NotCallable,
}

#[derive(Debug, Clone, Copy)]
enum CompoundOp {
    Add,
    Sub,
    Mult,
    Div,
    Mod,
    Pow,
}
