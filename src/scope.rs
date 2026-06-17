use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::RuntimeError;
use crate::gc::{GcRef, Remap, Trace};
use crate::value::Value;

pub type ValueRef = GcRef<Value>;

/// A single scope context — maps names to ValueRef (GcRef<Value>).
/// No RefValue wrapping needed since mutation goes through `GcHeap::with_mut`.
#[derive(Debug, Clone)]
pub struct ScopeContext {
    values: HashMap<String, ValueRef>,
}

impl Default for ScopeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeContext {
    pub fn new() -> Self {
        ScopeContext {
            values: HashMap::new(),
        }
    }

    /// Get a value by name. Returns None if not found in this scope.
    pub fn get_value(&self, key: &str) -> Option<ValueRef> {
        self.values.get(key).copied()
    }

    /// Declare a new value in this scope.
    /// Returns error if already declared in this scope.
    pub fn new_value(&mut self, key: &str, value: ValueRef) -> Result<ValueRef, RuntimeError> {
        debug_assert!(!key.is_empty(), "new_value: key must not be empty");
        debug_assert!(!value.is_null(), "new_value: value must not be null");
        if self.values.contains_key(key) {
            return Err(RuntimeError::new(format!(
                "Variable '{}' already declared in this scope",
                key
            )));
        }
        self.values.insert(key.to_string(), value);
        debug_assert!(
            self.values.contains_key(key),
            "new_value: key not stored after insert"
        );
        Ok(value)
    }

    /// Set an existing value by name. Returns error if not found.
    /// Replaces the GcRef in the map (the variable now points to a different value).
    pub fn set_value(&mut self, key: &str, value: ValueRef) -> Result<(), RuntimeError> {
        if self.values.contains_key(key) {
            self.values.insert(key.to_string(), value);
            Ok(())
        } else {
            Err(RuntimeError::new(format!(
                "Variable '{}' not declared",
                key
            )))
        }
    }

    /// Get all names declared in this scope
    pub fn keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    pub fn values_mut(&mut self) -> &mut HashMap<String, ValueRef> {
        &mut self.values
    }

    pub fn values(&self) -> &HashMap<String, ValueRef> {
        &self.values
    }
}

impl Trace for ScopeContext {
    fn trace(&self, visit: &mut dyn FnMut(usize)) {
        for v in self.values.values() {
            visit(v.offset());
        }
    }
}

impl Remap for ScopeContext {
    fn remap(&mut self, map: &[(usize, usize)]) {
        for v in self.values.values_mut() {
            if v.is_null() {
                continue;
            }
            for &(old, new) in map {
                if old == v.offset() {
                    *v = GcRef::from_offset(new);
                    break;
                }
            }
        }
    }
}

/// Scope manager: a stack of ScopeContexts (shared via Rc<RefCell<>> so that
/// closures capture a live view of the scope stack).
/// Implements static scoping with closure capture.
#[derive(Debug, Clone)]
pub struct ScopeManager {
    scopes: Vec<Rc<RefCell<ScopeContext>>>,
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeManager {
    pub fn new() -> Self {
        ScopeManager { scopes: Vec::new() }
    }

    /// Look up a value by name through all scopes, innermost first.
    /// Returns the ValueRef if found, None otherwise.
    pub fn get_value(&self, key: &str) -> Option<ValueRef> {
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.borrow().get_value(key) {
                return Some(val);
            }
        }
        None
    }

    /// Declare a new variable in the topmost (current) scope.
    pub fn new_top_level_value(
        &mut self,
        key: &str,
        value: ValueRef,
    ) -> Result<ValueRef, RuntimeError> {
        let top = self
            .scopes
            .last()
            .ok_or_else(|| RuntimeError::new("No active scope".to_string()))?;
        top.borrow_mut().new_value(key, value)
    }

    /// Set an existing variable (searching through scopes, innermost first).
    /// If not found, creates it in the top scope.
    pub fn set_value(&mut self, key: &str, value: ValueRef) -> Result<(), RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if scope.borrow().get_value(key).is_some() {
                return scope.borrow_mut().set_value(key, value);
            }
        }
        // If not found, declare in top scope
        self.new_top_level_value(key, value)?;
        Ok(())
    }

    /// Push a new scope onto the stack (entering a block).
    pub fn push_scope(&mut self, scope: Rc<RefCell<ScopeContext>>) {
        self.scopes.push(scope);
    }

    /// Push a new empty scope.
    pub fn push_new_scope(&mut self) {
        self.scopes.push(Rc::new(RefCell::new(ScopeContext::new())));
    }

    /// Pop the topmost scope (leaving a block).
    pub fn pop_scope(&mut self) {
        if !self.scopes.is_empty() {
            self.scopes.pop();
        }
    }

    /// Create a copy of the scope stack for closure capture.
    /// The new ScopeManager shares the same ScopeContext references
    /// (Rc<RefCell<>>), so mutations to variables are visible across closures.
    pub fn copy_scopes(&self) -> Self {
        ScopeManager {
            scopes: self.scopes.clone(),
        }
    }

    /// Get the number of active scopes
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}

impl Trace for ScopeManager {
    fn trace(&self, visit: &mut dyn FnMut(usize)) {
        for scope in &self.scopes {
            if let Ok(ctx) = scope.try_borrow() {
                ctx.trace(visit);
            }
        }
    }
}

impl Remap for ScopeManager {
    fn remap(&mut self, map: &[(usize, usize)]) {
        for scope in &self.scopes {
            if let Ok(mut ctx) = scope.try_borrow_mut() {
                ctx.remap(map);
            }
        }
    }
}
