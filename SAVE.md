# Shæll Interpreter — Save State

**Date:** 2026-06-11
**Status:** Green — 365 tests pass, clean build, format clean

## Quick Resume

```bash
cd /home/ai-man/Documents/shaell-project
cargo build --release          # builds in <2s
cargo test                     # 365 tests, all pass
./target/release/shaell        # REPL mode
./target/release/shaell foo.ae  # run script
```

## Architecture

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Module declarations
├── gc.rs             # Custom GC: byte-level heap, mark-compact, cycle-safe
├── token.rs          # Lexer tokens (logos)
├── lexer.rs          # Lexer wrapper
├── ast.rs            # AST types
├── parser.rs         # Recursive descent parser + precedence climbing
├── value.rs          # Value enum (GcRef<Value>, no Rc for values)
├── scope.rs          # ScopeManager + ScopeContext (Rc only for scope sharing)
├── interpreter.rs    # Tree-walking interpreter
├── pipe.rs           # External program execution & piping
├── builtin.rs        # Standard library
├── error.rs          # ParseError + RuntimeError
├── repl.rs           # REPL + file execution
└── tests/            # Unit test submodules (value, scope, pipe, lexer, error, token_ast)
tests/
└── integration_tests.rs  # 238 script-level tests
.github/workflows/
├── ci.yml            # Build, test, coverage, clippy, fmt
└── release.yml       # Cross-platform release on tag v*
```

## Key Design Decisions

1. **GC Heap** (`gc.rs`): Own byte buffer (`Vec<u8>`, 256KB initial), no Rust allocator for objects. Mark-sweep-compact with `Trace`/`Remap` traits. Cycle-safe. `GcRef<T>` is a Copy offset handle.

2. **Value System** (`value.rs`): `Value` enum with variants Number, String, BString, Bool, Null, File, Table, Function, NativeFunction. `ValueRef = GcRef<Value>`. Zero `Rc<RefCell<>>` for value storage.

3. **Scope** (`scope.rs`): `Rc<RefCell<ScopeContext>>` only for sharing scope hash maps across closures. This is NOT value storage — it's structural sharing of the scope chain.

4. **Parser** (`parser.rs`): Hand-written recursive descent with precedence climbing. Whitespace is tokenized (not skipped) for correct string content handling.

5. **Interpreter**: Direct tree-walking. Functions use in-place state save/restore for calls instead of sub-interpreters.

## Test Coverage (approximate)

| Module | Line % |
|--------|--------|
| token.rs | 100% |
| lexer.rs | 100% |
| error.rs | 100% |
| ast.rs | 96% |
| scope.rs | 93% |
| interpreter.rs | 93% |
| parser.rs | 84% |
| value.rs | 83% |
| builtin.rs | 66% |
| pipe.rs | 51% |
| main.rs | 0% (I/O) |
| repl.rs | 0% (I/O) |

## What's Left / Next Steps

- **Piping**: Advanced piping (`pipe { out -> ..., err -> ... }`) partially implemented, needs completion
- **File builtins**: `read()`, `append()`, `delete()`, `size()`, `exists()` methods on File values — stubbed but need implementation
- **Builtin registration**: Builtins are registered in global scope but some (Number/String methods) need to be callable via colon syntax
- **GC integration depth**: Top-level `Value` is GC-allocated, but `String`/`Vec<u8>` inside Value variants still use Rust's allocator for their internal buffers. To fully eliminate Rust allocator usage, these would need custom GC-backed string/buffer types
- **Concurrency**: Not implemented (explicitly won't-have per MoSCoW)
- **`define_args`**: Parsed but command-line args not wired through

## Commands

```bash
cargo build --release        # Release build
cargo test                   # All 365 tests
cargo llvm-cov --summary-only  # Coverage report
cargo fmt                    # Format code
cargo clippy                 # Lint
```


claude --resume 1602e460-7b39-42c1-954b-7e0e4655676a