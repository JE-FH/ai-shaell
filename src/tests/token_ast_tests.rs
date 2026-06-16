use crate::ast::*;
use crate::token::Token;
use crate::token::TokenStream;

// ============================================================
// AST Display tests
// ============================================================

#[test]
fn test_ast_display_expressions() {
    // Just verify Display impls don't panic
    let _ = format!("{}", Expr::Number(42.0));
    let _ = format!("{}", Expr::Boolean(true));
    let _ = format!("{}", Expr::Null);
    let _ = format!("{}", Expr::Identifier("x".to_string()));
    let _ = format!(
        "{}",
        Expr::Add(Box::new(Expr::Number(1.0)), Box::new(Expr::Number(2.0)))
    );
    let _ = format!("{}", Expr::UnaryNeg(Box::new(Expr::Number(5.0))));
    let _ = format!(
        "{}",
        Expr::Assign(
            Box::new(Expr::Identifier("x".to_string())),
            Box::new(Expr::Number(1.0))
        )
    );
}

// ============================================================
// Token Display tests
// ============================================================

#[test]
fn test_token_display_all_variants() {
    // Test Display on a sampling of each category
    let tokens = [
        Token::If,
        Token::Then,
        Token::Else,
        Token::End,
        Token::While,
        Token::Do,
        Token::Foreach,
        Token::For,
        Token::In,
        Token::Return,
        Token::Break,
        Token::Fn,
        Token::Let,
        Token::False,
        Token::True,
        Token::Null,
        Token::Throw,
        Token::Try,
        Token::With,
        Token::Pipe,
        Token::Exec,
        Token::DefineArgs,
        Token::Assign,
        Token::PlusEq,
        Token::MinusEq,
        Token::MultEq,
        Token::DivEq,
        Token::ModEq,
        Token::PowEq,
        Token::Into,
        Token::Deref,
        Token::Dollar,
        Token::DQuote,
        Token::Interpolation,
        Token::EscapedDollar,
        Token::EscapedBackslash,
        Token::EscapedNewline,
        Token::LParen,
        Token::RParen,
        Token::LCurl,
        Token::RCurl,
        Token::LSquare,
        Token::RSquare,
        Token::Colon,
        Token::Comma,
        Token::Lambda,
        Token::Pow,
        Token::Mult,
        Token::Div,
        Token::Mod,
        Token::Plus,
        Token::Minus,
        Token::LNot,
        Token::LT,
        Token::GT,
        Token::GEQ,
        Token::LEQ,
        Token::EQ,
        Token::NEQ,
        Token::LAnd,
        Token::LOr,
        Token::Whitespace,
        Token::Number(2.5),
        Token::Identifier("x".to_string()),
    ];
    for token in &tokens {
        let _ = format!("{}", token);
    }
}

// ============================================================
// Token edge cases
// ============================================================

#[test]
fn test_token_clone_eq() {
    let t1 = Token::Identifier("x".to_string());
    let t2 = t1.clone();
    assert_eq!(t1, t2);
    assert_ne!(t1, Token::Number(42.0));
}

// ============================================================
// StringLit Display test
// ============================================================

#[test]
fn test_string_lit_display() {
    let s = StringLit {
        parts: vec![
            StringPart::Text("hello".to_string()),
            StringPart::Newline,
            StringPart::Interpolation(Box::new(Expr::Identifier("x".to_string()))),
        ],
    };
    let _ = format!("{}", s);
}

// ============================================================
// Token: Span test
// ============================================================

#[test]
fn test_token_stream_spans() {
    let stream = TokenStream::new("let x = 42");
    let tokens: Vec<_> = stream.collect();
    // Should have tokens with spans
    assert!(!tokens.is_empty());
    for t in &tokens {
        assert!(t.span.end >= t.span.start);
    }
}

// ============================================================
// AST: Exhaustive Display on all expression variants
// ============================================================

#[test]
fn test_ast_display_all_expr_variants() {
    let n = Box::new(Expr::Number(1.0));
    let s = Box::new(Expr::Number(2.0));
    let id = Box::new(Expr::Identifier("x".to_string()));
    let exprs: Vec<Expr> = vec![
        Expr::Number(1.0),
        Expr::String_(StringLit { parts: vec![] }),
        Expr::Boolean(true),
        Expr::Null,
        Expr::Object(vec![]),
        Expr::Identifier("x".to_string()),
        Expr::Deref(id.clone()),
        Expr::IndexColon(id.clone(), "field".to_string()),
        Expr::IndexSubscript(id.clone(), n.clone()),
        Expr::Call(id.clone(), vec![]),
        Expr::UnaryLNot(id.clone()),
        Expr::UnaryNeg(id.clone()),
        Expr::UnaryPlus(id.clone()),
        Expr::Pow(n.clone(), s.clone()),
        Expr::Mult(n.clone(), s.clone()),
        Expr::Div(n.clone(), s.clone()),
        Expr::Mod(n.clone(), s.clone()),
        Expr::Add(n.clone(), s.clone()),
        Expr::Sub(n.clone(), s.clone()),
        Expr::LT(n.clone(), s.clone()),
        Expr::LEQ(n.clone(), s.clone()),
        Expr::GT(n.clone(), s.clone()),
        Expr::GEQ(n.clone(), s.clone()),
        Expr::EQ(n.clone(), s.clone()),
        Expr::NEQ(n.clone(), s.clone()),
        Expr::LAnd(n.clone(), s.clone()),
        Expr::LOr(n.clone(), s.clone()),
        Expr::Assign(n.clone(), s.clone()),
        Expr::PlusEq(n.clone(), s.clone()),
        Expr::MinusEq(n.clone(), s.clone()),
        Expr::MultEq(n.clone(), s.clone()),
        Expr::DivEq(n.clone(), s.clone()),
        Expr::ModEq(n.clone(), s.clone()),
        Expr::PowEq(n.clone(), s.clone()),
        Expr::Let("x".to_string()),
        Expr::LetAssign("x".to_string(), n.clone()),
        Expr::AnonFnDef(AnonFunctionDef {
            params: vec![],
            body: FunctionBody::Lambda(n.clone()),
        }),
        Expr::Try(vec![]),
        Expr::ProgProgram(ProgProgram {
            program: Box::new(Expr::Identifier("cmd".to_string())),
            args: vec![],
            pipe_target: None,
            pipe_expr: None,
        }),
    ];
    for e in &exprs {
        let _ = format!("{}", e);
    }
}
