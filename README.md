# Shæll Interpreter

A Rust implementation of the **Shæll** programming language — an intuitive shell scripting language designed as a better alternative to Bash. Based on the [Shæll whitepaper](./shæll%20whitepaper.txt) by CS22SW46, Aalborg University (Spring 2022).

## Quick Start

```bash
# Build
cargo build --release

# Run a script
./target/release/shaell myscript.ae

# Start interactive REPL
./target/release/shaell
```

## Language Overview

Shæll is a shell scripting language that improves on Bash with C-like syntax, real types, and intuitive keywords. It prioritises readability while keeping the power of shell scripting.

### Variables & Types

```
let name = "World"         # String
let count = 42             # Number (real, not just integers)
let flag = true            # Bool
let nothing = null         # Null
let path = @"some/file"    # File (lazy handle)

name = "Hello"             # Reassignment
count += 1                 # Compound assignment
```

Built-in types: **Number** (float64), **String** (UTF-16), **BString** (binary), **Bool**, **Null**, **File**, **Table**, **Function**.

### Strings

```
"Hello, ${name}!"          # Interpolation
"Hello " + "World"         # Concatenation
"Ha" * 3                   # Repetition → "HaHaHa"
"multi\nline"              # Escape sequences
```

### Control Flow

```
if score >= 90 then
    "A"
else
    "B"
end

while count > 0 do
    count = count - 1
end

for i = 0, i < 10, i += 1 do
    echo(i)
end

foreach value in collection do
    process(value)
end

foreach key, value in table do
    "key: ${key}, value: ${value}"
end
```

### Functions

```
fn add(a, b)
    return a + b
end

fn square(x) => x * x      # Lambda shorthand

let triple = fn (x) => x * 3  # Anonymous function

# Closures — functions capture their declaring scope
fn make_counter()
    let n = 0
    fn count()
        n = n + 1
        return n
    end
    return count
end
```

### Tables

```
let scores = { alice = 95, bob = 82 }
let array  = { [0] = "zero", [1] = "one" }

scores:alice        # → 95   (colon access)
scores["alice"]     # → 95   (bracket access)
array[0]            # → "zero"

scores:carol = 78   # Add/modify entry
```

### External Programs

```
# Simple execution
exec echo with ("Hello from echo")

# Simple piping (stdout → stdin)
exec echo with ("hello world") -> exec grep with ("world")

# Capture output into a variable
exec whoami with () -> let username

# Force file lookup (bypass function/variable shadowing)
exec @"echo" with ("raw echo call")
```

### Advanced Piping

```
pipe curl with ("https://example.com")
    out    -> exec grep with ("pattern")
    err    -> let errors
    status -> let exit_code
end
```

### Error Handling

```
let result = try
    risky_operation()
end

if result:status == 0 then
    "Success: ${result:value}"
else
    "Failed: ${result:error}"
end

# Explicit throw
if something_wrong then
    throw "something went wrong"
end
```

### Program Arguments

```
define_args args
    input_file,
    output_file
end

let in_file = @"${args:input_file}"
"Processing ${args:input_file} → ${args:output_file}"
```

### File Operations

```
let f = @"data.txt"

f:append("new content\n")
f:readToEnd()
f:size()
f:exists()
f:delete()
```

## Running Tests

```bash
cargo test
```

55 tests cover variables, all operators, control flow, functions, closures, tables, string operations, error handling, and more.

## Project Structure

```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Module declarations
├── token.rs          # Lexer token definitions (logos)
├── lexer.rs          # Lexer wrapper
├── ast.rs            # AST node types (30+ expression kinds)
├── parser.rs         # Recursive descent parser with precedence climbing
├── value.rs          # Runtime type system (IValue trait + 9 concrete types)
├── scope.rs          # ScopeManager + ScopeContext (static scoping)
├── interpreter.rs    # Tree-walking interpreter (Visitor pattern)
├── pipe.rs           # External program execution & piping
├── builtin.rs        # Standard library (Number/String methods, test framework)
├── error.rs          # ParseError + RuntimeError with stack traces
├── repl.rs           # REPL mode with multi-line input, .shællrc support
└── tests.rs          # 55 integration tests
```

## Implementation Notes

- **Parsing**: `logos` crate for lexing + hand-written recursive descent parser (no ANTLR/Java dependency)
- **Interpretation**: Direct AST tree-walking (matching the reference C# implementation's approach)
- **Memory**: `Rc<RefCell<>>` for shared mutable state (tables, closures)
- **Scope**: Stack of hash maps with copy-on-capture for closures
- **Piping**: `std::process::Command` for cross-platform external program execution

## Requirements

- Rust 1.96+ (stable)
- No external runtime dependencies beyond the Rust standard library and `logos` (compiled at build time)

## Reference

See `NOTES.md` for a detailed language reference and `PLAN.md` for the implementation architecture.
