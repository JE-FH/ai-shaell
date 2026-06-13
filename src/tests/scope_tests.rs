#[cfg(test)]
mod scope_tests {
    use crate::gc::GcHeap;
    use crate::scope::{ScopeContext, ScopeManager};
    use crate::value::*;

    // ============================================================
    // Scope unit tests (Rust-level)
    // ============================================================

    fn make_number(heap: &GcHeap, n: f64) -> ValueRef {
        heap.allocate(Value::Number(n))
    }

    #[test]
    fn test_scope_new_value_and_get() {
        let heap = GcHeap::new();
        let mut scope = ScopeContext::new();
        let val = make_number(&heap, 42.0);
        scope.new_value("x", val).unwrap();
        let got = scope.get_value("x");
        assert!(got.is_some());
        assert_eq!(
            heap.with_ref(got.unwrap(), |v| v.to_number().unwrap()),
            42.0
        );
    }

    #[test]
    fn test_scope_duplicate_declaration_error() {
        let heap = GcHeap::new();
        let mut scope = ScopeContext::new();
        scope.new_value("x", make_number(&heap, 1.0)).unwrap();
        assert!(scope.new_value("x", make_number(&heap, 2.0)).is_err());
    }

    #[test]
    fn test_scope_get_nonexistent() {
        let scope = ScopeContext::new();
        assert!(scope.get_value("nonexistent").is_none());
    }

    #[test]
    fn test_scope_manager_push_pop() {
        let heap = GcHeap::new();
        let mut sm = ScopeManager::new();
        sm.push_new_scope();
        sm.new_top_level_value("x", make_number(&heap, 5.0))
            .unwrap();
        assert_eq!(sm.depth(), 1);
        sm.pop_scope();
        assert_eq!(sm.depth(), 0);
    }

    #[test]
    fn test_scope_manager_shadowing() {
        let heap = GcHeap::new();
        let mut sm = ScopeManager::new();
        sm.push_new_scope();
        sm.new_top_level_value("x", make_number(&heap, 1.0))
            .unwrap();
        sm.push_new_scope();
        sm.new_top_level_value("x", make_number(&heap, 2.0))
            .unwrap();

        let val = sm.get_value("x").unwrap();
        assert_eq!(heap.with_ref(val, |v| v.to_number().unwrap()), 2.0);

        sm.pop_scope();
        let val = sm.get_value("x").unwrap();
        assert_eq!(heap.with_ref(val, |v| v.to_number().unwrap()), 1.0);
    }

    #[test]
    fn test_scope_manager_set_existing() {
        let heap = GcHeap::new();
        let mut sm = ScopeManager::new();
        sm.push_new_scope();
        sm.new_top_level_value("x", make_number(&heap, 1.0))
            .unwrap();
        sm.set_value("x", make_number(&heap, 99.0)).unwrap();
        let val = sm.get_value("x").unwrap();
        assert_eq!(heap.with_ref(val, |v| v.to_number().unwrap()), 99.0);
    }

    #[test]
    fn test_scope_manager_set_creates_if_missing() {
        let heap = GcHeap::new();
        let mut sm = ScopeManager::new();
        sm.push_new_scope();
        sm.set_value("x", make_number(&heap, 42.0)).unwrap();
        let val = sm.get_value("x").unwrap();
        assert_eq!(heap.with_ref(val, |v| v.to_number().unwrap()), 42.0);
    }

    // ============================================================
    // SCOPE REMAINING GAPS
    // ============================================================

    #[test]
    fn ctx_set_value_updates() {
        let heap = GcHeap::new();
        let mut c = ScopeContext::new();
        c.new_value("x", make_number(&heap, 1.0)).unwrap();
        c.set_value("x", make_number(&heap, 99.0)).unwrap();
        assert_eq!(
            heap.with_ref(c.get_value("x").unwrap(), |v| v.to_number().unwrap()),
            99.0
        );
    }

    #[test]
    fn ctx_set_value_not_found_errs() {
        let heap = GcHeap::new();
        assert!(ScopeContext::new()
            .set_value("nonexistent", heap.allocate(Value::Null))
            .is_err());
    }

    #[test]
    fn ctx_keys() {
        let heap = GcHeap::new();
        let mut c = ScopeContext::new();
        c.new_value("a", make_number(&heap, 0.0)).unwrap();
        c.new_value("b", make_number(&heap, 0.0)).unwrap();
        let k = c.keys();
        assert_eq!(k.len(), 2);
        assert!(k.contains(&"a".into()) && k.contains(&"b".into()));
    }

    #[test]
    fn mgr_get_nonexistent_none() {
        assert!(ScopeManager::new().get_value("x").is_none());
    }

    #[test]
    fn mgr_set_in_parent() {
        let heap = GcHeap::new();
        let mut m = ScopeManager::new();
        m.push_new_scope();
        m.new_top_level_value("x", make_number(&heap, 1.0)).unwrap();
        m.push_new_scope();
        m.set_value("x", make_number(&heap, 99.0)).unwrap();
        m.pop_scope();
        assert_eq!(
            heap.with_ref(m.get_value("x").unwrap(), |v| v.to_number().unwrap()),
            99.0
        );
    }

    #[test]
    fn mgr_new_top_no_scopes() {
        let heap = GcHeap::new();
        let r = ScopeManager::new().new_top_level_value("x", heap.allocate(Value::Null));
        assert!(r.is_err() && r.unwrap_err().message.contains("No active scope"));
    }

    #[test]
    fn mgr_copy_scopes() {
        let heap = GcHeap::new();
        let mut m = ScopeManager::new();
        m.push_new_scope();
        m.new_top_level_value("x", make_number(&heap, 5.0)).unwrap();
        assert_eq!(
            heap.with_ref(m.copy_scopes().get_value("x").unwrap(), |v| v
                .to_number()
                .unwrap()),
            5.0
        );
    }

    #[test]
    fn mgr_pop_empty_noop() {
        let mut m = ScopeManager::new();
        m.pop_scope();
        assert_eq!(m.depth(), 0);
    }
}
