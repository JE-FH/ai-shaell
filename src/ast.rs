/// Abstract Syntax Tree for the Shæll language.
/// Follows the grammar defined in Appendix C.2 of the whitepaper.
/// Top-level program
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub args: Option<ProgramArgs>,
    pub statements: Vec<Stmt>,
}

/// Program arguments definition
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramArgs {
    pub table_name: String,
    pub arg_names: Vec<String>,
}

/// Statements
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    If(IfStmt),
    For(ForLoop),
    While(WhileLoop),
    Foreach(ForeachLoop),
    ForeachKeyValue(ForeachKeyValueLoop),
    Return(Box<Expr>),
    Throw(Box<Expr>),
    Break,
    FunctionDef(FunctionDef),
    PipeProgram(PipeProgram),
}

/// Expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // === Literals ===
    Number(f64),
    String_(StringLit),
    Boolean(bool),
    Null,
    Object(Vec<(ObjectField, Box<Expr>)>),

    // === Identifiers ===
    Identifier(String),

    // === Operations ===
    Deref(Box<Expr>),                     // @expr
    IndexColon(Box<Expr>, String),        // expr:identifier
    IndexSubscript(Box<Expr>, Box<Expr>), // expr[expr]
    Call(Box<Expr>, Vec<Expr>),           // expr(args)
    UnaryLNot(Box<Expr>),                 // !expr
    UnaryNeg(Box<Expr>),                  // -expr
    UnaryPlus(Box<Expr>),                 // +expr
    Pow(Box<Expr>, Box<Expr>),            // expr ** expr
    Mult(Box<Expr>, Box<Expr>),           // expr * expr
    Div(Box<Expr>, Box<Expr>),            // expr / expr
    Mod(Box<Expr>, Box<Expr>),            // expr % expr
    Add(Box<Expr>, Box<Expr>),            // expr + expr
    Sub(Box<Expr>, Box<Expr>),            // expr - expr
    LT(Box<Expr>, Box<Expr>),             // expr < expr
    LEQ(Box<Expr>, Box<Expr>),            // expr <= expr
    GT(Box<Expr>, Box<Expr>),             // expr > expr
    GEQ(Box<Expr>, Box<Expr>),            // expr >= expr
    EQ(Box<Expr>, Box<Expr>),             // expr == expr
    NEQ(Box<Expr>, Box<Expr>),            // expr != expr
    LAnd(Box<Expr>, Box<Expr>),           // expr && expr
    LOr(Box<Expr>, Box<Expr>),            // expr || expr

    // === Assignment ===
    Assign(Box<Expr>, Box<Expr>),  // expr = expr
    PlusEq(Box<Expr>, Box<Expr>),  // expr += expr
    MinusEq(Box<Expr>, Box<Expr>), // expr -= expr
    MultEq(Box<Expr>, Box<Expr>),  // expr *= expr
    DivEq(Box<Expr>, Box<Expr>),   // expr /= expr
    ModEq(Box<Expr>, Box<Expr>),   // expr %= expr
    PowEq(Box<Expr>, Box<Expr>),   // expr **= expr

    // === Let declaration ===
    Let(String),                  // let x
    LetAssign(String, Box<Expr>), // let x = expr

    // === Function expressions ===
    AnonFnDef(AnonFunctionDef),

    // === Try expression ===
    Try(Vec<Stmt>), // try stmts end

    // === External program ===
    ProgProgram(ProgProgram),
}

/// A string literal with parsed content
#[derive(Debug, Clone, PartialEq)]
pub struct StringLit {
    pub parts: Vec<StringPart>,
}

/// Parts of a string literal
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Text(String),
    Newline,
    EscapedDollar,
    EscapedBackslash,
    Interpolation(Box<Expr>),
}

/// Field in an object literal
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectField {
    Identifier(String),   // foo = expr
    Subscript(Box<Expr>), // [expr] = expr
}

/// If statement
#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Box<Expr>,
    pub then_body: Vec<Stmt>,
    pub else_body: Option<Vec<Stmt>>,
}

/// For loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoop {
    pub init: Box<Expr>,
    pub condition: Box<Expr>,
    pub update: Box<Expr>,
    pub body: Vec<Stmt>,
}

/// While loop
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoop {
    pub condition: Box<Expr>,
    pub body: Vec<Stmt>,
}

/// Foreach loop (iterate over values)
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachLoop {
    pub var: String,
    pub collection: Box<Expr>,
    pub body: Vec<Stmt>,
}

/// Foreach key-value loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForeachKeyValueLoop {
    pub key_var: String,
    pub value_var: String,
    pub collection: Box<Expr>,
    pub body: Vec<Stmt>,
}

/// Function definition
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: FunctionBody,
}

/// Anonymous function definition
#[derive(Debug, Clone, PartialEq)]
pub struct AnonFunctionDef {
    pub params: Vec<String>,
    pub body: FunctionBody,
}

/// Function body can be a block or a lambda expression
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Block(Vec<Stmt>),
    Lambda(Box<Expr>),
}

/// Simple program execution + piping
#[derive(Debug, Clone, PartialEq)]
pub struct ProgProgram {
    pub program: Box<Expr>,
    pub args: Vec<Expr>,
    pub pipe_target: Option<Box<PipeTarget>>,
    pub pipe_expr: Option<Box<Expr>>,
}

/// Pipe target: either another program or a pipe program
#[derive(Debug, Clone, PartialEq)]
pub enum PipeTarget {
    ProgProgram(ProgProgram),
    PipeProgram(PipeProgram),
}

/// Advanced pipe program
#[derive(Debug, Clone, PartialEq)]
pub struct PipeProgram {
    pub program: Box<Expr>,
    pub args: Vec<Expr>,
    pub descriptors: Vec<PipeDesc>,
}

/// Pipe descriptor: maps a stream name to a target
#[derive(Debug, Clone, PartialEq)]
pub enum PipeDesc {
    IntoPipe(String, PipeTarget), // stream -> pipeTarget
    OntoExpr(String, Box<Expr>),  // stream -> expr
}

impl Program {
    pub fn new(args: Option<ProgramArgs>, statements: Vec<Stmt>) -> Self {
        Program { args, statements }
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Number(n) => write!(f, "{}", n),
            Expr::String_(s) => write!(f, "\"{}\"", s),
            Expr::Boolean(b) => write!(f, "{}", b),
            Expr::Null => write!(f, "null"),
            Expr::Object(_) => write!(f, "{{...}}"),
            Expr::Identifier(id) => write!(f, "{}", id),
            Expr::Deref(e) => write!(f, "@({})", e),
            Expr::IndexColon(e, key) => write!(f, "({}):{}", e, key),
            Expr::IndexSubscript(e, idx) => write!(f, "({})[{}]", e, idx),
            Expr::Call(e, args) => write!(f, "{}({:?})", e, args),
            Expr::UnaryLNot(e) => write!(f, "!({})", e),
            Expr::UnaryNeg(e) => write!(f, "-({})", e),
            Expr::UnaryPlus(e) => write!(f, "+({})", e),
            Expr::Pow(l, r) => write!(f, "({} ** {})", l, r),
            Expr::Mult(l, r) => write!(f, "({} * {})", l, r),
            Expr::Div(l, r) => write!(f, "({} / {})", l, r),
            Expr::Mod(l, r) => write!(f, "({} % {})", l, r),
            Expr::Add(l, r) => write!(f, "({} + {})", l, r),
            Expr::Sub(l, r) => write!(f, "({} - {})", l, r),
            Expr::LT(l, r) => write!(f, "({} < {})", l, r),
            Expr::LEQ(l, r) => write!(f, "({} <= {})", l, r),
            Expr::GT(l, r) => write!(f, "({} > {})", l, r),
            Expr::GEQ(l, r) => write!(f, "({} >= {})", l, r),
            Expr::EQ(l, r) => write!(f, "({} == {})", l, r),
            Expr::NEQ(l, r) => write!(f, "({} != {})", l, r),
            Expr::LAnd(l, r) => write!(f, "({} && {})", l, r),
            Expr::LOr(l, r) => write!(f, "({} || {})", l, r),
            Expr::Assign(l, r) => write!(f, "({} = {})", l, r),
            Expr::PlusEq(l, r) => write!(f, "({} += {})", l, r),
            Expr::MinusEq(l, r) => write!(f, "({} -= {})", l, r),
            Expr::MultEq(l, r) => write!(f, "({} *= {})", l, r),
            Expr::DivEq(l, r) => write!(f, "({} /= {})", l, r),
            Expr::ModEq(l, r) => write!(f, "({} %= {})", l, r),
            Expr::PowEq(l, r) => write!(f, "({} **= {})", l, r),
            Expr::Let(id) => write!(f, "let {}", id),
            Expr::LetAssign(id, e) => write!(f, "let {} = {}", id, e),
            Expr::AnonFnDef(_) => write!(f, "fn(...) ... end"),
            Expr::Try(_) => write!(f, "try ... end"),
            Expr::ProgProgram(_) => write!(f, "exec ..."),
        }
    }
}

impl std::fmt::Display for StringLit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for part in &self.parts {
            match part {
                StringPart::Text(t) => write!(f, "{}", t)?,
                StringPart::Newline => write!(f, "\\n")?,
                StringPart::EscapedDollar => write!(f, "\\$")?,
                StringPart::EscapedBackslash => write!(f, "\\\\")?,
                StringPart::Interpolation(e) => write!(f, "${{{}}}", e)?,
            }
        }
        Ok(())
    }
}
