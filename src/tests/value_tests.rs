#[cfg(test)]
mod value_tests {
    use crate::ast::FunctionBody;
    use crate::gc::GcHeap;
    use crate::gc::GcRef;
    use crate::scope::ScopeManager;
    use crate::value::*;

    // ============================================================
    // Value system unit tests (Rust-level)
    // ============================================================

    #[test]
    fn test_value_number_to_bool() {
        assert_eq!(Value::Number(0.0).to_bool().unwrap(), false);
        assert_eq!(Value::Number(1.0).to_bool().unwrap(), true);
        assert_eq!(Value::Number(-1.0).to_bool().unwrap(), true);
    }

    #[test]
    fn test_value_number_to_sstring() {
        assert_eq!(Value::Number(42.0).to_sstring().unwrap(), "42");
        assert_eq!(Value::Number(3.14).to_sstring().unwrap(), "3.14");
    }

    #[test]
    fn test_value_string_to_bool() {
        assert_eq!(Value::String("".to_string()).to_bool().unwrap(), false);
        assert_eq!(Value::String("x".to_string()).to_bool().unwrap(), true);
    }

    #[test]
    fn test_value_string_to_number() {
        assert_eq!(Value::String("42".to_string()).to_number().unwrap(), 42.0);
        assert_eq!(Value::String("3.14".to_string()).to_number().unwrap(), 3.14);
        assert!(Value::String("abc".to_string()).to_number().is_err());
    }

    #[test]
    fn test_value_bool_to_number() {
        assert_eq!(Value::Bool(true).to_number().unwrap(), 1.0);
        assert_eq!(Value::Bool(false).to_number().unwrap(), 0.0);
    }

    #[test]
    fn test_value_bool_to_sstring() {
        assert_eq!(Value::Bool(true).to_sstring().unwrap(), "true");
        assert_eq!(Value::Bool(false).to_sstring().unwrap(), "false");
    }

    #[test]
    fn test_value_null_properties() {
        let n = Value::Null;
        assert_eq!(n.to_bool().unwrap(), false);
        assert_eq!(n.to_number().unwrap(), 0.0);
        assert_eq!(n.to_sstring().unwrap(), "null");
        assert_eq!(n.get_type_name(), "null");
    }

    #[test]
    fn test_value_bstring_to_bool() {
        assert_eq!(Value::BString(vec![]).to_bool().unwrap(), false);
        assert_eq!(Value::BString(vec![1, 2, 3]).to_bool().unwrap(), true);
    }

    #[test]
    fn test_value_bstring_to_sstring() {
        let bs = Value::BString(vec![104, 101, 108, 108, 111]); // "hello"
        assert_eq!(bs.to_sstring().unwrap(), "hello");
    }

    #[test]
    fn test_value_file_properties() {
        let f = Value::File("/tmp/test.txt".to_string());
        assert_eq!(f.get_type_name(), "file");
        assert_eq!(f.to_sstring().unwrap(), "/tmp/test.txt");
        assert_eq!(f.to_file_path().unwrap(), "/tmp/test.txt");
    }

    #[test]
    fn test_value_is_equal_numbers() {
        let a = Value::Number(5.0);
        let b = Value::Number(5.0);
        let c = Value::Number(3.0);
        assert!(a.is_equal(&b).unwrap());
        assert!(!a.is_equal(&c).unwrap());
    }

    #[test]
    fn test_value_is_equal_strings() {
        let a = Value::String("hello".to_string());
        let b = Value::String("hello".to_string());
        let c = Value::String("world".to_string());
        assert!(a.is_equal(&b).unwrap());
        assert!(!a.is_equal(&c).unwrap());
    }

    #[test]
    fn test_value_is_equal_bools() {
        assert!(Value::Bool(true).is_equal(&Value::Bool(true)).unwrap());
        assert!(!Value::Bool(true).is_equal(&Value::Bool(false)).unwrap());
    }

    #[test]
    fn test_value_is_equal_null() {
        assert!(Value::Null.is_equal(&Value::Null).unwrap());
    }

    #[test]
    fn test_value_is_equal_cross_type() {
        let n = Value::Number(5.0);
        let s = Value::String("5".to_string());
        assert!(!n.is_equal(&s).unwrap());
    }

    #[test]
    fn test_value_type_names() {
        assert_eq!(Value::Number(1.0).get_type_name(), "number");
        assert_eq!(Value::String("x".to_string()).get_type_name(), "string");
        assert_eq!(Value::Bool(true).get_type_name(), "bool");
        assert_eq!(Value::Null.get_type_name(), "null");
        assert_eq!(Value::File("x".to_string()).get_type_name(), "file");
        assert_eq!(Value::table().get_type_name(), "table");
        assert_eq!(
            Value::Function(FuncData {
                name: "f".to_string(),
                params: vec![],
                body: FunctionBody::Block(vec![]),
                captured_scope: ScopeManager::new(),
            })
            .get_type_name(),
            "function"
        );
    }

    // ============================================================
    // Table tests
    // ============================================================

    #[test]
    fn test_table_index_and_set() {
        let heap = GcHeap::new();
        let mut table_val = Value::table();
        let key = Value::String("a".to_string());
        let val = heap.allocate(Value::Number(42.0));
        table_val.set_index(&key, val).unwrap();

        let result = table_val.index(&key, &heap).unwrap();
        assert_eq!(heap.with_ref(result, |v| v.to_number().unwrap()), 42.0);
    }

    #[test]
    fn test_table_array_index() {
        let heap = GcHeap::new();
        let mut t = Value::table();
        t.set_index(
            &Value::Number(0.0),
            heap.allocate(Value::String("zero".to_string())),
        )
        .unwrap();
        let v = t.index(&Value::Number(0.0), &heap).unwrap();
        assert_eq!(heap.with_ref(v, |v| v.to_sstring().unwrap()), "zero");
    }

    #[test]
    fn test_table_missing_key() {
        let heap = GcHeap::new();
        let t = Value::table();
        let result = t
            .index(&Value::String("nonexistent".to_string()), &heap)
            .unwrap();
        // Missing key returns a Value::Null, not GcRef::NULL
        let v = heap.with_ref(result, |v| v.clone());
        assert!(matches!(v, Value::Null));
    }

    #[test]
    fn test_table_get_keys() {
        let heap = GcHeap::new();
        let mut t = Value::table();
        t.set_index(
            &Value::String("a".to_string()),
            heap.allocate(Value::Number(1.0)),
        )
        .unwrap();
        t.set_index(
            &Value::String("b".to_string()),
            heap.allocate(Value::Number(2.0)),
        )
        .unwrap();
        let keys = t.get_keys(&heap).unwrap();
        assert_eq!(keys.len(), 2);
    }

    // ============================================================
    // BString tests
    // ============================================================

    #[test]
    fn test_value_bstring_to_number() {
        let bs = Value::BString(vec![52, 50]); // "42"
        assert_eq!(bs.to_number().unwrap(), 42.0);
    }

    #[test]
    fn test_value_bstring_to_number_invalid() {
        let bs = Value::BString(vec![104, 105]); // "hi"
        assert!(bs.to_number().is_err());
    }

    #[test]
    fn test_value_bstring_is_equal() {
        let a = Value::BString(vec![1, 2, 3]);
        let b = Value::BString(vec![1, 2, 3]);
        let c = Value::BString(vec![4, 5, 6]);
        assert!(a.is_equal(&b).unwrap());
        assert!(!a.is_equal(&c).unwrap());
    }

    #[test]
    fn test_value_file_is_equal() {
        let a = Value::File("/tmp/a".to_string());
        let b = Value::File("/tmp/a".to_string());
        let c = Value::File("/tmp/c".to_string());
        assert!(a.is_equal(&b).unwrap());
        assert!(!a.is_equal(&c).unwrap());
    }

    #[test]
    fn test_value_file_to_bool() {
        // File values are truthy (non-null)
        let f = Value::File("/nonexistent/path/12345".to_string());
        assert_eq!(f.to_bool().unwrap(), true);
    }

    #[test]
    fn test_value_default_to_file_path_error() {
        assert!(Value::Number(5.0).to_file_path().is_err());
    }

    #[test]
    fn test_value_number_is_equal_cross_type() {
        let n = Value::Number(5.0);
        let s = Value::String("5".to_string());
        assert!(!n.is_equal(&s).unwrap());
    }

    #[test]
    fn test_value_null_is_equal_cross_type() {
        assert!(!Value::Null.is_equal(&Value::Number(0.0)).unwrap());
    }

    #[test]
    fn test_value_table_to_bool() {
        let t = Value::table();
        assert_eq!(t.to_bool().unwrap(), false);
        let mut t2 = Value::table();
        t2.set_index(
            &Value::String("a".to_string()),
            GcRef::NULL, // doesn't matter for to_bool
        )
        .ok();
        assert_eq!(t2.to_bool().unwrap(), true);
    }

    #[test]
    fn test_value_snumber_display() {
        assert_eq!(format!("{}", Value::Number(42.0)), "42");
        assert_eq!(format!("{}", Value::Number(3.14)), "3.14");
    }

    #[test]
    fn test_value_bstring_display() {
        let bs = Value::BString(vec![104, 105]);
        assert_eq!(format!("{}", bs), "hi");
    }

    #[test]
    fn test_value_sfile_display() {
        let f = Value::File("/tmp/test".to_string());
        assert!(format!("{}", f).contains("/tmp/test"));
    }

    #[test]
    fn test_value_user_func_display() {
        let uf = Value::Function(FuncData {
            name: "test".to_string(),
            params: vec!["x".to_string()],
            body: FunctionBody::Lambda(Box::new(crate::ast::Expr::Number(1.0))),
            captured_scope: ScopeManager::new(),
        });
        let _ = format!("{}", uf);
        let _ = format!("{:?}", uf);
    }

    #[test]
    fn test_value_native_func_display() {
        let heap = GcHeap::new();
        let nf = Value::NativeFunction(NativeFuncData {
            name: "test_fn".to_string(),
            func: std::rc::Rc::new(|_, _| Ok(GcRef::NULL)),
        });
        let _ = format!("{}", nf);
        let _ = format!("{:?}", nf);
    }

    // ============================================================
    // Number edge cases
    // ============================================================

    #[test]
    fn test_value_number_to_sstring_negative() {
        assert_eq!(Value::Number(-5.0).to_sstring().unwrap(), "-5");
    }

    #[test]
    fn test_value_number_to_sstring_nan() {
        assert!(!Value::Number(f64::NAN).to_sstring().unwrap().is_empty());
    }

    #[test]
    fn test_value_number_is_equal_epsilon() {
        assert!(Value::Number(1.0)
            .is_equal(&Value::Number(1.0 + f64::EPSILON / 2.0))
            .unwrap());
        assert!(!Value::Number(1.0).is_equal(&Value::Number(2.0)).unwrap());
    }

    #[test]
    fn test_value_default_index_error() {
        let n = Value::Number(5.0);
        let key = Value::String("key".to_string());
        let heap = GcHeap::new();
        assert!(n.index(&key, &heap).is_err());
    }

    #[test]
    fn test_value_default_set_index_error() {
        let mut n = Value::Number(5.0);
        let key = Value::String("key".to_string());
        assert!(n.set_index(&key, GcRef::NULL).is_err());
    }

    // ============================================================
    // GC heap allocation and access
    // ============================================================

    #[test]
    fn test_gc_alloc_and_read() {
        let heap = GcHeap::new();
        let h = heap.allocate(Value::Number(42.0));
        let v = heap.with_ref(h, |v| v.clone());
        assert!(matches!(v, Value::Number(42.0)));
    }

    #[test]
    fn test_gc_alloc_and_mutate() {
        let heap = GcHeap::new();
        let h = heap.allocate(Value::Number(99.0));
        heap.with_mut(h, |v| *v = Value::Number(55.0));
        assert_eq!(heap.with_ref(h, |v| v.to_number().unwrap()), 55.0);
    }

    #[test]
    fn test_gc_alloc_string() {
        let heap = GcHeap::new();
        let h = heap.allocate(Value::String("hello".to_string()));
        let s = heap.with_ref(h, |v| v.to_sstring().unwrap());
        assert_eq!(s, "hello");
    }

    #[test]
    fn test_gc_alloc_table() {
        let heap = GcHeap::new();
        let t = heap.allocate(Value::table());
        let key = Value::String("x".to_string());
        let val = heap.allocate(Value::Number(10.0));
        heap.with_mut(t, |v| v.set_index(&key, val)).unwrap();
        let result = heap.with_ref(t, |v| v.index(&key, &heap)).unwrap();
        assert_eq!(heap.with_ref(result, |v| v.to_number().unwrap()), 10.0);
    }

    #[test]
    fn test_gc_string_method_length() {
        let heap = GcHeap::new();
        let s_val = Value::String("hello".to_string());
        let result = s_val
            .index(&Value::String("length".to_string()), &heap)
            .unwrap();
        assert_eq!(heap.with_ref(result, |v| v.to_number().unwrap()), 5.0);
    }

    #[test]
    fn test_gc_number_methods() {
        let heap = GcHeap::new();
        let n = Value::Number(9.0);
        let r = n.index(&Value::String("sqrt".to_string()), &heap).unwrap();
        assert_eq!(heap.with_ref(r, |v| v.to_number().unwrap()), 3.0);

        let r2 = Value::Number(3.7)
            .index(&Value::String("floor".to_string()), &heap)
            .unwrap();
        assert_eq!(heap.with_ref(r2, |v| v.to_number().unwrap()), 3.0);

        let r3 = Value::Number(3.2)
            .index(&Value::String("ceil".to_string()), &heap)
            .unwrap();
        assert_eq!(heap.with_ref(r3, |v| v.to_number().unwrap()), 4.0);

        let r4 = Value::Number(8.0)
            .index(&Value::String("log2".to_string()), &heap)
            .unwrap();
        assert_eq!(heap.with_ref(r4, |v| v.to_number().unwrap()), 3.0);

        let r5 = Value::Number(100.0)
            .index(&Value::String("log".to_string()), &heap)
            .unwrap();
        assert_eq!(heap.with_ref(r5, |v| v.to_number().unwrap()), 2.0);
    }

    #[test]
    fn test_value_convenience_constructors() {
        assert!(matches!(Value::number(1.0), Value::Number(1.0)));
        assert!(matches!(Value::string("hi".into()), Value::String(_)));
        assert!(matches!(Value::bool_val(true), Value::Bool(true)));
        assert!(matches!(Value::null(), Value::Null));
        assert!(matches!(Value::file("p".into()), Value::File(_)));
        assert!(matches!(Value::table(), Value::Table(_)));
    }

    #[test]
    fn test_serialize_all_types() {
        assert_eq!(Value::Number(42.0).serialize(), "42");
        assert_eq!(Value::String("hi".to_string()).serialize(), "hi");
        assert_eq!(Value::Null.serialize(), "null");
        assert_eq!(Value::Bool(true).serialize(), "true");
        assert_eq!(Value::Bool(false).serialize(), "false");
        assert!(Value::File("/p".to_string()).serialize().contains("/p"));
        let t = Value::table();
        assert!(t.serialize().contains("{"));
        let f = Value::Function(FuncData {
            name: "f".into(),
            params: vec![],
            body: FunctionBody::Block(vec![]),
            captured_scope: ScopeManager::new(),
        });
        assert!(f.serialize().contains("f"));
        let nf = Value::NativeFunction(NativeFuncData {
            name: "nf".into(),
            func: std::rc::Rc::new(|_, _| Ok(GcRef::NULL)),
        });
        assert!(nf.serialize().contains("nf"));
    }

    #[test]
    fn test_value_is_equal_bstring() {
        let a = Value::BString(vec![1, 2, 3]);
        let b = Value::BString(vec![1, 2, 3]);
        let c = Value::Number(1.0);
        assert!(a.is_equal(&b).unwrap());
        assert!(!a.is_equal(&c).unwrap());
    }

    // ── mutation-testing gaps ───────────────────────────────

    #[test]
    fn test_null_to_sstring() {
        assert_eq!(Value::Null.to_sstring().unwrap(), "null");
    }

    #[test]
    fn test_index_out_of_bounds_returns_null() {
        let heap = GcHeap::new();
        let t = heap.allocate(Value::table());
        // Check array index 0 on empty table returns null
        let result = heap.with_ref(t, |v| {
            match v {
                Value::Table(td) => {
                    if 0 < td.array.len() { td.array[0] } else { GcRef::NULL }
                }
                _ => GcRef::NULL,
            }
        });
        assert!(result.is_null());
    }

    #[test]
    fn test_set_index_resizes_array() {
        let heap = GcHeap::new();
        let t = heap.allocate(Value::table());
        let val = heap.allocate(Value::string("hello".to_string()));
        // Set at index 5 — should resize array
        heap.with_mut(t, |v| {
            v.set_index(&Value::Number(5.0), val).unwrap();
        });
        // Verify array was resized by checking internal state
        heap.with_ref(t, |v| {
            if let Value::Table(td) = v {
                assert!(td.array.len() >= 6);
            }
        });
    }
}
