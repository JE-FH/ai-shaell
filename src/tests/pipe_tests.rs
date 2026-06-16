use crate::ast::*;
use crate::pipe::resolve_program_path;

// ============================================================
// Pipe module unit tests
// ============================================================

#[test]
fn test_pipe_resolve_identifier() {
    let expr = Expr::Identifier("echo".to_string());
    let result = resolve_program_path(&expr);
    assert_eq!(result.unwrap(), "echo");
}

#[test]
fn test_pipe_resolve_string() {
    let expr = Expr::String_(StringLit {
        parts: vec![StringPart::Text("ls".to_string())],
    });
    let result = resolve_program_path(&expr);
    assert_eq!(result.unwrap(), "ls");
}

#[test]
fn test_pipe_resolve_deref_identifier() {
    let expr = Expr::Deref(Box::new(Expr::Identifier("myprog".to_string())));
    let result = resolve_program_path(&expr);
    assert_eq!(result.unwrap(), "myprog");
}

#[test]
fn test_pipe_resolve_deref_string() {
    let expr = Expr::Deref(Box::new(Expr::String_(StringLit {
        parts: vec![StringPart::Text("prog".to_string())],
    })));
    let result = resolve_program_path(&expr);
    assert_eq!(result.unwrap(), "prog");
}

#[test]
fn test_pipe_resolve_invalid() {
    let expr = Expr::Number(42.0);
    let result = resolve_program_path(&expr);
    assert!(result.is_err());
}

#[test]
fn test_pipe_resolve_deref_invalid_inner() {
    let expr = Expr::Deref(Box::new(Expr::Number(1.0)));
    let result = resolve_program_path(&expr);
    assert!(result.is_err());
}

fn s_expr(s: &str) -> Expr {
    Expr::String_(StringLit {
        parts: vec![StringPart::Text(s.into())],
    })
}

#[test]
fn resolve_id() {
    assert_eq!(
        resolve_program_path(&Expr::Identifier("ls".into())).unwrap(),
        "ls"
    );
}

#[test]
fn resolve_str() {
    assert_eq!(resolve_program_path(&s_expr("/bin/sh")).unwrap(), "/bin/sh");
}

#[test]
fn resolve_deref_str() {
    assert_eq!(
        resolve_program_path(&Expr::Deref(Box::new(s_expr("/usr/bin/env")))).unwrap(),
        "/usr/bin/env"
    );
}

#[test]
fn resolve_deref_id() {
    assert_eq!(
        resolve_program_path(&Expr::Deref(Box::new(Expr::Identifier("grep".into())))).unwrap(),
        "grep"
    );
}

#[test]
fn resolve_num_err() {
    assert!(resolve_program_path(&Expr::Number(42.0)).is_err());
}

#[test]
fn resolve_deref_num_err() {
    assert!(resolve_program_path(&Expr::Deref(Box::new(Expr::Number(1.0)))).is_err());
}

#[test]
fn exec_nonexistent_err() {
    let mut p = crate::parser::Parser::new("exec does_not_exist_xyz123 with ()");
    let prog = p.parse_program().expect("Parse failed");
    assert!(crate::interpreter::Interpreter::new()
        .execute_program(&prog)
        .is_err());
}

#[test]
fn exec_echo_exit_code() {
    let mut p = crate::parser::Parser::new("exec echo with (\"x\")");
    let prog = p.parse_program().expect("Parse failed");
    let mut interpreter = crate::interpreter::Interpreter::new();
    let r = interpreter.execute_program(&prog).expect("exec failed");
    let val = interpreter.heap.with_ref(r, |v| v.to_sstring().unwrap());
    assert_eq!(val, "0");
}

#[test]
fn exec_echo_pipe_capture() {
    let mut p = crate::parser::Parser::new("exec echo with (\"piped_val\") -> let out\nout");
    let prog = p.parse_program().expect("Parse failed");
    let mut interpreter = crate::interpreter::Interpreter::new();
    let r = interpreter.execute_program(&prog).expect("exec failed");
    let val = interpreter.heap.with_ref(r, |v| v.to_sstring().unwrap());
    assert!(val.contains("piped_val"));
}
