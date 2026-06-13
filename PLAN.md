# Shæll Interpreter — Rust Implementation Plan

## Architecture Decisions

### Parsing Strategy
- **Lexer**: `logos` crate for tokenization. Two modes: DEFAULT_MODE and STRING_MODE.
- **Parser**: Hand-written recursive descent. Gives full control over error messages, handles mode switching for string interpolation naturally, avoids external parser generator dependencies.

### Interpretation Strategy
- **Direct tree-walking interpreter** (like the C# reference). No intermediate representation or bytecode.
- **Visitor pattern**: Single `Interpreter` struct that recursively evaluates AST nodes.
- This keeps development time reasonable and follows the reference design.

### Why These Choices
- No ANTLR4 (Java-based, outside Rust ecosystem)
- No bytecode compilation (keeps it simple for v1)
- `logos` is fast, well-maintained, and handles multiple lexer modes
- Recursive descent is the most straightforward for this grammar

## Project Structure

```
shaell/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, CLI args, REPL loop
│   ├── lib.rs            # Module declarations
│   ├── token.rs          # Token enum (logos-derived)
│   ├── lexer.rs          # Lexer wrapping logos
│   ├── ast.rs            # All AST node types
│   ├── parser.rs         # Recursive descent parser
│   ├── value.rs          # IValue trait + all value types
│   ├── scope.rs          # ScopeContext + ScopeManager
│   ├── interpreter.rs    # Tree-walking interpreter
│   ├── pipe.rs           # Process execution + piping
│   ├── error.rs          # Error types with stack traces
│   ├── builtin.rs        # Standard library functions
│   └── repl.rs           # REPL mode
└── tests/
    ├── integration_test.rs
    └── scripts/          # .æ test scripts
```

## Dependencies (Cargo.toml)
```toml
[dependencies]
logos = "0.14"        # Lexer generator
```

No other external dependencies needed. We use only the Rust standard library:
- `std::process::Command` for external program execution
- `std::collections::HashMap` for scopes
- `std::fs` for file operations

## Implementation Phases

### Phase 1: Foundation
1. Initialize Cargo project
2. Define Token enum with logos
3. Implement lexer
4. Define AST types
5. Implement parser

### Phase 2: Core Runtime
6. Implement value type system (IValue, Number, String, Bool, etc.)
7. Implement scope management
8. Implement core interpreter (expressions, statements, control flow)

### Phase 3: Functions & Data
9. Implement function definitions, calls, closures
10. Implement tables and table operations
11. Implement file objects and dereference operator

### Phase 4: Shell Features
12. Implement external program execution (exec)
13. Implement simple piping (->)
14. Implement advanced piping (pipe/out/err/status/end)

### Phase 5: Polish
15. Implement error handling (try/throw)
16. Implement standard library (Number/String/File methods)
17. REPL mode, file execution, .shællrc
18. Program arguments (define_args)
19. String interpolation in lexer/parser

### Phase 6: Testing
20. Write integration tests using .æ scripts
21. Test all language features

## Key Design Patterns

### Value System
```rust
trait IValue {
    fn to_bool(&self) -> bool;
    fn to_number(&self) -> f64;
    fn to_sstring(&self) -> String;
    fn to_function(&self) -> Option<...>;
    fn to_table(&self) -> Option<...>;
    fn to_file(&self) -> Option<...>;
    fn get_type_name(&self) -> &str;
    fn unpack(&self) -> &dyn IValue;  // unwrap RefValue
    fn is_equal(&self, other: &dyn IValue) -> bool;
    fn serialize(&self) -> String;
}

// Concrete types: Number(f64), SString(String), BString(Vec<u8>),
// SBool(bool), SNull, SFile(String), UserFunc {...}, UserTable {...}, RefValue {...}
```

### Scope
```rust
struct ScopeContext {
    values: HashMap<String, Rc<RefCell<dyn IValue>>>,
}

struct ScopeManager {
    scopes: Vec<Rc<RefCell<ScopeContext>>>,
}
```

### Interpreter
```rust
struct Interpreter {
    scope_manager: ScopeManager,
    global_scope: Rc<RefCell<ScopeContext>>,
    should_return: bool,
    return_value: Option<Rc<RefCell<dyn IValue>>>,
}

impl Interpreter {
    fn interpret(&mut self, stmts: &[Stmt]) -> Result<Value, Error>;
    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, Error>;
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<(), Error>;
}
```

## Error Handling Strategy
- StackTracedError: accumulates stack trace as it propagates
- SemanticError: for type errors, undefined variables, etc.
- Safe visit pattern: wrap each AST visit with error context

## Memory Model
- Use `Rc<RefCell<...>>` for shared mutable values (closures, tables)
- Tables are reference types (shared ownership)
- Primitives are value types (copied on assignment/call)
