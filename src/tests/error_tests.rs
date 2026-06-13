#[cfg(test)]
mod error_tests {
    use crate::error::*;

    // ============================================================
    // Error type tests
    // ============================================================

    #[test]
    fn test_runtime_error_display() {
        let err = RuntimeError::new("test error".to_string());
        let display = format!("{}", err);
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_runtime_error_with_trace() {
        let err = RuntimeError::new("base error".to_string())
            .with_trace("at function f".to_string())
            .with_trace("at line 5".to_string());
        let display = format!("{}", err);
        assert!(display.contains("base error"));
        assert!(display.contains("at line 5"));
        assert!(display.contains("at function f"));
    }

    #[test]
    fn test_parse_error_display() {
        let err = ParseError::UnexpectedToken {
            expected: "'end'".to_string(),
            found: "'}'".to_string(),
            span: 0..1,
        };
        let display = format!("{}", err);
        assert!(display.contains("'end'"));
        assert!(display.contains("'}'"));
    }

    #[test]
    fn test_parse_error_eof_display() {
        let err = ParseError::UnexpectedEof {
            expected: "'end'".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("'end'"));
        assert!(display.contains("end of file"));
    }

    // ============================================================
    // Custom error Display
    // ============================================================

    #[test]
    fn test_parse_error_custom_display() {
        let err = ParseError::Custom("test message".to_string());
        assert_eq!(format!("{}", err), "test message");
    }

    // ============================================================
    // ERROR REMAINING GAPS
    // ============================================================

    #[test]
    fn runtime_new_empty_traces() {
        let e = RuntimeError::new("msg".into());
        assert_eq!(e.message, "msg");
        assert!(e.traces.is_empty());
    }
    #[test]
    fn runtime_add_trace() {
        let mut e = RuntimeError::new("".into());
        e.add_trace("a".into());
        e.add_trace("b".into());
        assert_eq!(e.traces, vec!["a", "b"]);
    }
    #[test]
    fn runtime_display_no_traces() {
        assert_eq!(
            format!("{}", RuntimeError::new("simple".into())),
            "Error: simple\n"
        );
    }
    #[test]
    fn runtime_display_with_traces() {
        let e = RuntimeError::new("err".into())
            .with_trace("l1".into())
            .with_trace("l2".into());
        let d = format!("{}", e);
        assert!(d.contains("Error: err") && d.contains("at l2") && d.contains("at l1"));
    }
    #[test]
    fn parse_unexpected_token_display() {
        let e = ParseError::UnexpectedToken {
            expected: "id".into(),
            found: "num".into(),
            span: 0..1,
        };
        assert_eq!(format!("{}", e), "Expected id but found num");
    }
    #[test]
    fn parse_unexpected_eof_display() {
        let e = ParseError::UnexpectedEof {
            expected: "end".into(),
        };
        assert_eq!(format!("{}", e), "Expected end but reached end of file");
    }
}
