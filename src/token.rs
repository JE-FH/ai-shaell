use logos::Logos;

/// All tokens in the Shæll language.
/// Single-mode lexer: string parsing context is handled by the parser.
#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"#[^\n]*")] // Skip single-line comments (# ...)
#[logos(skip r"/\*[^*]*\*+(?:[^/*][^*]*\*+)*/")] // Skip multi-line comments
pub enum Token {
    /// Whitespace and newlines (filtered by parser between tokens)
    #[regex(r"[ \t\r\n]+")]
    Whitespace,
    // === Keywords (priority must exceed Identifier regex) ===
    #[token("if")]
    If,
    #[token("then")]
    Then,
    #[token("else")]
    Else,
    #[token("end")]
    End,
    #[token("while")]
    While,
    #[token("do")]
    Do,
    #[token("foreach")]
    Foreach,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("return")]
    Return,
    #[token("break")]
    Break,
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("not")]
    Not,
    #[token("false")]
    False,
    #[token("true")]
    True,
    #[token("null")]
    Null,
    #[token("throw")]
    Throw,
    #[token("try")]
    Try,
    #[token("with")]
    With,
    #[token("pipe")]
    Pipe,
    #[token("exec")]
    Exec,
    #[token("define_args")]
    DefineArgs,

    // === Delimiters ===
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LCurl,
    #[token("}")]
    RCurl,
    #[token("[")]
    LSquare,
    #[token("]")]
    RSquare,
    #[token(":")]
    Colon,
    #[token(",")]
    Comma,

    // === Operators ===
    #[token("=>")]
    Lambda,
    #[token("**")]
    Pow,
    #[token("*")]
    Mult,
    #[token("/")]
    Div,
    #[token("%")]
    Mod,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("!")]
    Bang,
    #[token("<")]
    LT,
    #[token(">")]
    GT,
    #[token(">=")]
    GEQ,
    #[token("<=")]
    LEQ,
    #[token("==")]
    EQ,
    #[token("!=")]
    NEQ,
    #[token("&&")]
    LAnd,
    #[token("||")]
    LOr,
    #[token("=")]
    Assign,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    MultEq,
    #[token("/=")]
    DivEq,
    #[token("%=")]
    ModEq,
    #[token("**=")]
    PowEq,
    #[token("->")]
    Into,
    #[token("@")]
    Deref,
    #[token("$")]
    Dollar,

    // === String-related (handled contextually by parser) ===
    #[token("\"")]
    DQuote,
    #[token("${")]
    Interpolation,
    #[token(r"\$")]
    EscapedDollar,
    #[token(r"\\")]
    EscapedBackslash,
    #[token(r"\n")]
    EscapedNewline,

    // === Literals (lower priority than keywords) ===
    #[regex(r"[0-9]+\.[0-9]+|[0-9]+", priority = 0, callback = |lex| lex.slice().parse::<f64>().unwrap_or(0.0))]
    Number(f64),

    #[regex(r"[a-zA-Z_.$][a-zA-Z0-9_.$]*", priority = 0, callback = |lex| lex.slice().to_string())]
    Identifier(String),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::If => write!(f, "'if'"),
            Token::Then => write!(f, "'then'"),
            Token::Else => write!(f, "'else'"),
            Token::End => write!(f, "'end'"),
            Token::While => write!(f, "'while'"),
            Token::Do => write!(f, "'do'"),
            Token::Foreach => write!(f, "'foreach'"),
            Token::For => write!(f, "'for'"),
            Token::In => write!(f, "'in'"),
            Token::Return => write!(f, "'return'"),
            Token::Break => write!(f, "'break'"),
            Token::Fn => write!(f, "'fn'"),
            Token::Let => write!(f, "'let'"),
            Token::False => write!(f, "'false'"),
            Token::True => write!(f, "'true'"),
            Token::Null => write!(f, "'null'"),
            Token::Throw => write!(f, "'throw'"),
            Token::Try => write!(f, "'try'"),
            Token::With => write!(f, "'with'"),
            Token::Pipe => write!(f, "'pipe'"),
            Token::Exec => write!(f, "'exec'"),
            Token::DefineArgs => write!(f, "'define_args'"),
            Token::LParen => write!(f, "'('"),
            Token::RParen => write!(f, "')'"),
            Token::LCurl => write!(f, "'{{'"),
            Token::RCurl => write!(f, "'}}'"),
            Token::LSquare => write!(f, "'['"),
            Token::RSquare => write!(f, "']'"),
            Token::Colon => write!(f, "':'"),
            Token::Comma => write!(f, "','"),
            Token::Lambda => write!(f, "'=>'"),
            Token::Pow => write!(f, "'**'"),
            Token::Mult => write!(f, "'*'"),
            Token::Div => write!(f, "'/'"),
            Token::Mod => write!(f, "'%'"),
            Token::Plus => write!(f, "'+'"),
            Token::Minus => write!(f, "'-'"),
            Token::Bang => write!(f, "'!'"),
            Token::Not => write!(f, "'not'"),
            Token::LT => write!(f, "'<'"),
            Token::GT => write!(f, "'>'"),
            Token::GEQ => write!(f, "'>='"),
            Token::LEQ => write!(f, "'<='"),
            Token::EQ => write!(f, "'=='"),
            Token::NEQ => write!(f, "'!='"),
            Token::LAnd => write!(f, "'&&'"),
            Token::LOr => write!(f, "'||'"),
            Token::Assign => write!(f, "'='"),
            Token::PlusEq => write!(f, "'+='"),
            Token::MinusEq => write!(f, "'-='"),
            Token::MultEq => write!(f, "'*='"),
            Token::DivEq => write!(f, "'/='"),
            Token::ModEq => write!(f, "'%='"),
            Token::PowEq => write!(f, "'**='"),
            Token::Into => write!(f, "'->'"),
            Token::Deref => write!(f, "'@'"),
            Token::Dollar => write!(f, "'$'"),
            Token::DQuote => write!(f, "\"\\\"\""),
            Token::Interpolation => write!(f, "'${{'"),
            Token::EscapedDollar => write!(f, "'\\\\$'"),
            Token::EscapedBackslash => write!(f, "'\\\\\\\\'"),
            Token::EscapedNewline => write!(f, "'\\\\n'"),
            Token::Number(n) => write!(f, "{}", n),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::Whitespace => write!(f, " "),
        }
    }
}

/// A located token
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: std::ops::Range<usize>,
}

/// Token iterator wrapping logos lexer
pub struct TokenStream<'source> {
    lexer: logos::Lexer<'source, Token>,
}

impl<'source> TokenStream<'source> {
    pub fn new(source: &'source str) -> Self {
        TokenStream {
            lexer: Token::lexer(source),
        }
    }
}

impl<'source> Iterator for TokenStream<'source> {
    type Item = SpannedToken;

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|result| match result {
            Ok(token) => SpannedToken {
                token,
                span: self.lexer.span().clone(),
            },
            Err(_) => {
                // Skip invalid tokens
                SpannedToken {
                    token: Token::Null,
                    span: self.lexer.span().clone(),
                }
            }
        })
    }
}
