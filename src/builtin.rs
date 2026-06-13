use crate::error::RuntimeError;
use crate::gc::GcHeap;
use crate::value::*;

/// Register all standard library functions into a global scope table
pub fn register_builtins(heap: &GcHeap, table_ref: &ValueRef) {
    register_number_functions(heap, table_ref);
    register_string_functions(heap, table_ref);
    register_test_functions(heap, table_ref);
}

fn register_number_functions(heap: &GcHeap, parent: &ValueRef) {
    let number_table = heap.allocate(Value::table());

    add_native(heap, &number_table, "sqrt", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let n = heap.with_ref(args[0], |v| v.to_number())?;
        Ok(heap.allocate(Value::Number(n.sqrt())))
    });

    add_native(heap, &number_table, "floor", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let n = heap.with_ref(args[0], |v| v.to_number())?;
        Ok(heap.allocate(Value::Number(n.floor())))
    });

    add_native(heap, &number_table, "ceil", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let n = heap.with_ref(args[0], |v| v.to_number())?;
        Ok(heap.allocate(Value::Number(n.ceil())))
    });

    add_native(heap, &number_table, "log2", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let n = heap.with_ref(args[0], |v| v.to_number())?;
        Ok(heap.allocate(Value::Number(n.log2())))
    });

    add_native(heap, &number_table, "log", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let n = heap.with_ref(args[0], |v| v.to_number())?;
        Ok(heap.allocate(Value::Number(n.log10())))
    });

    let key_val = Value::String("Number".to_string());
    heap.with_mut(*parent, |t| t.set_index(&key_val, number_table))
        .ok();
}

fn register_string_functions(heap: &GcHeap, parent: &ValueRef) {
    let string_table = heap.allocate(Value::table());

    add_native(heap, &string_table, "length", |heap, args| {
        if args.is_empty() {
            return Err(RuntimeError::new("missing self".to_string()));
        }
        let s = heap.with_ref(args[0], |v| v.to_sstring())?;
        Ok(heap.allocate(Value::Number(s.chars().count() as f64)))
    });

    add_native(heap, &string_table, "substring", |heap, args| {
        if args.len() < 3 {
            return Err(RuntimeError::new(
                "substring requires start and end indices".to_string(),
            ));
        }
        let s = heap.with_ref(args[0], |v| v.to_sstring())?;
        let start = heap.with_ref(args[1], |v| v.to_number())? as usize;
        let end = heap.with_ref(args[2], |v| v.to_number())? as usize;
        let chars: Vec<char> = s.chars().collect();
        let result: String = chars
            .iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();
        Ok(heap.allocate(Value::String(result)))
    });

    let key_val = Value::String("String".to_string());
    heap.with_mut(*parent, |t| t.set_index(&key_val, string_table))
        .ok();
}

fn register_test_functions(heap: &GcHeap, table_ref: &ValueRef) {
    add_native(heap, table_ref, "assert", |heap, args| {
        if args.len() < 2 {
            return Err(RuntimeError::new(
                "assert requires condition and message".to_string(),
            ));
        }
        let cond = heap.with_ref(args[0], |v| v.to_bool().unwrap_or(false));
        if !cond {
            let msg = heap.with_ref(args[1], |v| v.to_sstring().unwrap_or_default());
            return Err(RuntimeError::new(msg));
        }
        Ok(heap.allocate(Value::String("ok".to_string())))
    });

    add_native(heap, table_ref, "assertType", |heap, args| {
        if args.len() < 2 {
            return Err(RuntimeError::new(
                "assertType requires value and type name".to_string(),
            ));
        }
        let type_name = heap.with_ref(args[1], |v| v.to_sstring())?;
        let actual = heap.with_ref(args[0], |v| v.get_type_name().to_string());
        if actual != type_name {
            return Err(RuntimeError::new(format!(
                "Expected type '{}' but got '{}'",
                type_name, actual
            )));
        }
        Ok(heap.allocate(Value::String("ok".to_string())))
    });

    add_native(heap, table_ref, "describe", |heap, args| {
        if args.len() < 2 {
            return Err(RuntimeError::new(
                "describe requires name and function".to_string(),
            ));
        }
        let name = heap.with_ref(args[0], |v| v.to_sstring())?;
        // Call the function with no args (via interpreter, but here we just check callable)
        let result = if heap.with_ref(args[1], |v| {
            matches!(v, Value::NativeFunction(_) | Value::Function(_))
        }) {
            // We can't actually call user functions from here without interpreter
            // For native functions, try to call
            if heap.with_ref(args[1], |v| matches!(v, Value::NativeFunction(_))) {
                // Get the native fn and call it
                let nf = heap.with_ref(args[1], |v| match v {
                    Value::NativeFunction(n) => n.func.clone(),
                    _ => unreachable!(),
                });
                nf(heap, &[])
            } else {
                Err(RuntimeError::new(
                    "Cannot call user function from builtin".to_string(),
                ))
            }
        } else {
            Err(RuntimeError::new("Not callable".to_string()))
        };

        match result {
            Ok(_) => {
                println!("  PASS: {}", name);
                Ok(heap.allocate(Value::String("ok".to_string())))
            }
            Err(err) => {
                println!("  FAIL: {} - {}", name, err.message);
                Err(err)
            }
        }
    });

    add_native(heap, table_ref, "tableEqual", |heap, args| {
        if args.len() < 3 {
            return Err(RuntimeError::new(
                "tableEqual requires table1, table2, and test name".to_string(),
            ));
        }
        let t1 = &args[0];
        let t2 = &args[1];
        let test_name = heap.with_ref(args[2], |v| v.to_sstring())?;
        let v1 = heap.with_ref(*t1, |v| v.clone());
        let v2 = heap.with_ref(*t2, |v| v.clone());
        let eq = v1.is_equal(&v2).unwrap_or(false);
        if !eq {
            return Err(RuntimeError::new(test_name));
        }
        Ok(heap.allocate(Value::String("ok".to_string())))
    });

    add_native(heap, table_ref, "tableEquivalent", |heap, args| {
        if args.len() < 3 {
            return Err(RuntimeError::new(
                "tableEquivalent requires table1, table2, and test name".to_string(),
            ));
        }
        let t1 = &args[0];
        let t2 = &args[1];
        let test_name = heap.with_ref(args[2], |v| v.to_sstring())?;
        let v1 = heap.with_ref(*t1, |v| v.clone());
        let v2 = heap.with_ref(*t2, |v| v.clone());
        let eq = v1.is_equal(&v2).unwrap_or(false);
        if !eq {
            return Err(RuntimeError::new(test_name));
        }
        Ok(heap.allocate(Value::String("ok".to_string())))
    });
}

fn add_native(
    heap: &GcHeap,
    table: &ValueRef,
    name: &str,
    func: impl Fn(&GcHeap, &[ValueRef]) -> Result<ValueRef, RuntimeError> + 'static,
) {
    let native_func = NativeFuncData {
        name: name.to_string(),
        func: std::rc::Rc::new(func),
    };
    let native_ref = heap.allocate(Value::NativeFunction(native_func));
    let key = Value::String(name.to_string());
    heap.with_mut(*table, |t| t.set_index(&key, native_ref))
        .ok();
}

/// Populate a `ScopeContext` with the builtins library table as 'A'.
pub fn populate_global_scope(heap: &GcHeap, scope: &mut crate::scope::ScopeContext) {
    let builtins_table = heap.allocate(Value::table());
    register_builtins(heap, &builtins_table);
    scope.new_value("A", builtins_table).ok();
}
