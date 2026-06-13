# Shæll Language Reference Notes

## Source
Based on "Shæll: The intuitive shell programming language" whitepaper by CS22SW46, Aalborg University, Spring 2022.

## Design Philosophy
- Improve on Bash for scripting (readability over brevity)
- Use English keywords instead of cryptic symbols
- C-like syntax familiar to most programmers
- Whitespace is NOT significant
- Dynamically typed with real types (not "everything is a string")
- Static scoping
- Prioritize readability, then writability, then reliability

---

## 1. Lexical Structure

### 1.1 Tokens (DEFAULT_MODE)
```
Keywords: if, then, else, end, while, do, foreach, for, in, return, break, fn, let
          false, true, null, throw, try, with, pipe, exec, define_args
Operators: ( ) { } [ ] : @ => * ** / % + - ! < > >= <= == != && || = , += -= *= /= %= **= ->
String: " -> pushMode(STRING_MODE)
STRINGCLOSEBRACE: } -> popMode (only in string interpolation context)
Identifier: [a-zA-Z_.$][a-zA-Z0-9_.$]*
Number: [0-9]+('.'[0-9]+)?
Comment: # ~('\n')* (single line), /* ... */ (multi-line)
Whitespace: (' ' | '\t' | '\r' | '\n')+ -> skip
Lambda: =>
```

### 1.2 String Mode Tokens (STRING_MODE)
```
ESCAPEDINTERPOLATION: \$
ESCAPEDESCAPE: \\
NEWLINE: \n
INTERPOLATION: ${ -> pushMode(DEFAULT_MODE)
END_STRING: " -> popMode
TEXT: ~('"'|'$'|'\\')+
```

### 1.3 Operator Precedence (lowest to highest)
| Precedence | Operators | Associativity |
|-----------|-----------|---------------|
| 0 | @ (dereference) | right-to-left |
| 1 | : (index), [] (subscript), () (call) | left-to-right |
| 2 | !, ~, unary - | right-to-left |
| 3 | ** | |
| 4 | *, /, % | left-to-right |
| 5 | +, - | left-to-right |
| 6 | <, <=, >, >= | left-to-right |
| 7 | ==, != | left-to-right |
| 8 | && | left-to-right |
| 9 | \|\| | left-to-right |
| 10 | -> (pipe) | |
| 11 | = | right-to-left |
| 12 | +=, -=, *=, /=, %=, **= | right-to-left |
| 13 | , (comma) | left-to-right |

---

## 2. Types

### 2.1 Primitive Types
- **Bool**: `true` / `false`
- **Number**: Any real number (signed/unsigned, floating point). Unlike Bash, supports decimals natively.
- **String** (SString): UTF-16 encoded text string
- **BString**: Binary string (from file I/O)
- **null**: The null value

### 2.2 Reference/Composite Types
- **File**: Lazy file handle. Created via `@"path"`. Represents a file path, not the actual file content.
- **Function**: User-defined functions and closures (static scoping). Also standard library functions.
- **Table**: Key-value collection like Lua tables. Keys can be any type. Also serve as arrays.

### 2.3 Type Conversions (Implicit)
- `Number + Number` → Number (arithmetic)
- `String + String` → String (concatenation)
- `Number + String` → String (concatenation, number converted to string)
- `String + Number` → String (concatenation)
- `Number * Number` → Number (multiplication)
- `String * Number` → String (repetition, e.g., "Hi" * 3 = "HiHiHi")
- `Number * String` → String (repetition)
- Unary `+` on String converts to Number: `+"123"` → 123
- Unary `-` on String converts to Number and negates

### 2.4 File Object
Lazy-loaded file handle. Opening happens on first operation.
```
Functions on File:
  read(amount: Number, offset: Number) → BString
  readToEnd() → BString
  append(data: String)
  delete()
  size() → Number
  exists() → Bool
```

---

## 3. Variables and Assignment

### 3.1 Declaration
```
let variable = 5          # Declare and initialize
let variable              # Declare without initialization (null)
variable = "Hello"        # Reassign
```

### 3.2 Scope Rules
- Static (lexical) scoping
- Block-structured: each block introduces a new scope
- `let` declares in current scope
- Assignment without `let` updates existing variable in nearest enclosing scope
- If variable doesn't exist in any scope, assignment creates it in current scope
- Identifiers not found in any scope evaluate to a File (via `F[[x]]`)
- Variables and functions share the same namespace (cannot have same name)

### 3.3 Compound Assignment
```
x += 1    # x = x + 1
x -= 1    # x = x - 1
x *= 2    # x = x * 2
x /= 2    # x = x / 2
x %= 2    # x = x % 2
x **= 2   # x = x ** 2
```

---

## 4. Tables and Arrays

### 4.1 Table Literal
```
let t = { foo = "H", bar = "e" }
let arr = { [1] = 32, [3] = 45, [4] = 28 }
let mixed = { [0] = 32, foo = "H", [1] = "hej" }
```

### 4.2 Indexing
```
table:key           # Static field access (colon notation)
table["key"]        # Dynamic string key access
table[index]        # Numeric index access
array:0             # ERROR - colon can't be used with numeric keys
```

### 4.3 Assignment to Table Fields
```
table:key = value
table[index] = value
```

---

## 5. Control Structures

### 5.1 If/Else
```
if condition then
    # statements
else
    # statements
end
```
Note: No `elseif` in the base spec (noted as missing feature).

Conditions: Bool or Number (0 = false, non-zero = true)

### 5.2 While Loop
```
while condition do
    # statements
end
```

### 5.3 For Loop (C-style)
```
for init, condition, update do
    # statements
end
```
Example: `for i = 0, i < 10, i += 1 do ... end`

### 5.4 Foreach Loop
```
foreach value in collection do
    # statements
end

foreach key, value in collection do
    # statements
end
```

### 5.5 Break
`break` keyword is a defined token but was not fully implemented in reference. We should implement it.

---

## 6. Functions

### 6.1 Declaration
```
fn name (arg1, arg2, arg3)
    # function body
    return value
end
```

### 6.2 Call
```
name(arg1, arg2)
```

### 6.3 Parameter Passing
- Primitive types (Number, String, Bool, null): call-by-value (copied)
- Tables: call-by-value but the value IS a reference, so modifications to table contents affect the original
- Functions: call-by-value (function reference is copied)

### 6.4 Return
```
return expression
```
If no return, function returns the value of the last expression? (Actually returns null based on reference code)

### 6.5 Closures (Static Scoping)
- Functions capture the scope at declaration time
- Variables declared after function declaration in the same scope ARE accessible
- Variables declared in nested scopes are NOT accessible from the function

### 6.6 Anonymous Functions
```
let myfunction = fn ()
    return 2 + 2
end

callfunc(fn () exec echo with ("Hello") end)
```

### 6.7 Lambda / Inline Functions
```
fn add(a, b) => a + b
let f = fn (x) => (x ** x):sqrt()
```

---

## 7. External Program Execution

### 7.1 Simple Execution
```
exec programName with (arg1, arg2, ...)
```
- `programName` is evaluated to a file (via @ operator or identifier lookup)
- Looks in relative path first, then PATH environment variable
- Can be shadowed by user-defined functions/variables
- Use `@"program"` to force file interpretation: `exec @"echo" with ("hello")`

### 7.2 Simple Piping
```
exec echo with ("hello") -> exec grep with ("world")
exec echo with ("hello") -> let output
exec echo with ("hello") -> table:key
```

### 7.3 Advanced Piping
```
pipe curl with ("https://google.com")
    out -> exec grep with ("Feeling Lucky") -> let grep_out
    err -> exec grep with ("404") -> let error404_grep
    status -> let status
end
```
Pipes available: `out` (stdout), `err` (stderr), `status` (exit code)
- Evaluated depth-first, sequentially
- If status is non-zero and not explicitly captured, throws an error
- Can nest: `out -> pipe ... end`

---

## 8. Error Handling

### 8.1 Try Expression
```
let res = try
    # code that might throw
end
```
Returns a table with fields:
- `status`: Number (0 = success, non-zero for external programs = exit code)
- `error`: Any (the thrown error value)
- `value`: Any (the last expression value if successful)

### 8.2 Throw
```
throw expression
```
Throws an error that propagates up until caught by a try block.

---

## 9. String Features

### 9.1 String Literals
```
"Hello World"
"Line1\nLine2"
```

### 9.2 String Interpolation
```
"3 plus 3 is ${3 + 3}"   # → "3 plus 3 is 6"
```

### 9.3 String Operations
- Concatenation: `"Hello " + "World"` → `"Hello World"`
- Repetition: `"Hi" * 3` → `"HiHiHi"`
- Length: `"foo":length()` → 3
- Substring: `"hello":substring(1, 3)` → `"el"`

---

## 10. Program Arguments

### 10.1 define_args
```
define_args args
    first,
    second,
    last
end
```
Creates a table `args` with positional arguments as named fields and array entries.
- `args:first` equals `args[0]`
- Can only be used once, at the beginning of a script

---

## 11. Math Library
```
num:sqrt()    # Square root
num:floor()   # Floor rounding
num:ceil()    # Ceil rounding
num:log2()    # Log base 2
num:log()     # Log base 10
```

---

## 12. Grammar (Full)

### 12.1 Program
```
prog: programArgs stmts | stmts;
stmts: stmt*;
```

### 12.2 Statements
```
stmt: ifStmt
    | forLoop
    | whileLoop
    | returnStatement
    | functionDefinition
    | foreach
    | foreachKeyValue
    | throwStatement
    | pipeProgram
    | expr
    ;
```

### 12.3 Expressions (with precedence)
```
expr:
    // Primary (unsorted by precedence)
    DQUOTE strcontent* END_STRING    # StringLiteralExpr
    | LET IDENTIFIER                 # LetExpr (bare let)
    | NUMBER                         # NumberExpr
    | NULL                           # NullExpr
    | boolean                        # BooleanExpr
    | TRY stmts END                  # TryExpr
    | IDENTIFIER                     # IdentifierExpr
    | LPAREN expr RPAREN             # Parenthesis
    | LCURL (objfields ASSIGN expr (COMMA objfields ASSIGN expr)*)? RCURL  # ObjectLiteral
    | progProgram                    # ProgProgramExpr
    
    // Sorted by precedence
    | <assoc=right> DEREF expr       # DerefExpr
    | expr COLON IDENTIFIER          # IdentifierIndexExpr
    | expr LSQUACKET expr RSQUACKET  # SubScriptExpr
    | expr LPAREN innerArgList RPAREN # FunctionCallExpr
    | <assoc=right> LNOT expr        # LnotExpr
    | <assoc=right> MINUS expr       # NegExpr
    | <assoc=right> PLUS expr        # PosExpr (unary plus)
    | expr POW expr                  # PowExpr
    | expr MULT expr                 # MultExpr
    | expr DIV expr                  # DivExpr
    | expr MOD expr                  # ModExpr
    | expr PLUS expr                 # AddExpr
    | expr MINUS expr                # MinusExpr
    | expr LT expr                   # LTExpr
    | expr LEQ expr                  # LEQExpr
    | expr GT expr                   # GTExpr
    | expr GEQ expr                  # GEQExpr
    | expr EQ expr                   # EQExpr
    | expr NEQ expr                  # NEQExpr
    | expr LAND expr                 # LANDExpr
    | expr LOR expr                  # LORExpr
    | <assoc=right> expr ASSIGN expr # AssignExpr
    | <assoc=right> expr PLUSEQ expr # PlusEqExpr
    | ... (other compound assignments)
    | anonFunctionDefinition         # AnonFnDefinition
    ;
```

### 12.4 Piping
```
pipeTarget: progProgram | pipeProgram;

progProgram:
    PROG expr (WITH LPAREN innerArgList RPAREN)? (INTO (pipeTarget | expr))?;

pipeProgram:
    PIPE expr (WITH LPAREN innerArgList RPAREN)? pipeDesc* END;

pipeDesc:
    IDENTIFIER INTO pipeTarget    # IntoDesc
    | IDENTIFIER INTO expr         # OntoDesc
    ;
```

### 12.5 Control Flow
```
ifStmt: IF expr THEN stmts (ELSE stmts)? END;
forLoop: FOR expr COMMA expr COMMA expr DO stmts END;
foreach: FOREACH IDENTIFIER IN expr DO stmts END;
foreachKeyValue: FOREACH IDENTIFIER COMMA IDENTIFIER IN expr DO stmts END;
whileLoop: WHILE expr DO stmts END;
```

### 12.6 Functions
```
functionDefinition: FUNCTION IDENTIFIER LPAREN innerFormalArgList RPAREN functionBody;
anonFunctionDefinition: FUNCTION LPAREN innerFormalArgList RPAREN functionBody;
functionBody: stmts END | LAMBDA expr;
returnStatement: RETURN expr;
```

### 12.7 Program Args
```
programArgs: ARGS IDENTIFIER innerFormalArgList END;
```

### 12.8 String Content
```
strcontent:
    NEWLINE                  # NewLine
    | ESCAPEDINTERPOLATION   # EscapedInterpolation
    | ESCAPEDESCAPE          # EscapedEscape
    | INTERPOLATION expr STRINGCLOSEBRACE  # Interpolation
    | TEXT                   # StringLiteral
    ;
```

---

## 13. Operational Semantics (Key Points)

### 13.1 Environment-Store Model
- `EnvV = Identifier ∪ {next} ⇀ Loc` — maps identifiers to locations
- `Sto = Loc ⇀ Values` — maps locations to stored values
- Static scoping: functions store `(stmts × EnvV × Params)` in the store

### 13.2 Variable Rules
- **VAR-DEC**: Evaluate expression, store in next location, update env and store
- **VAR-1**: Lookup identifier in env, get value from store
- **VAR-2**: If identifier not in env, use semantic function F to map to a File
- **ASSIGN**: Evaluate expression, update existing location in store

### 13.3 Operator Semantics
- **ADD**: Number+Number=arithmetic, String+String=concat, Number+String=concat
- **MULT**: Number*Number=multiply, String*Number=repeat string
- **AND**: Short-circuit evaluation
- **DEREF**: Converts String to File

### 13.4 Function Call
1. Evaluate actual parameters
2. Lookup function in env (gets body, captured env, formal params)
3. Create new scope on top of captured scope
4. Bind formal params to actual values in new scope
5. Execute function body
6. Return value

### 13.5 External Program Execution
- `eval(File, Params, StdIn) → Number` — executes file with args and stdin, returns exit code
- stdout goes to special variable in env
- Piping chains stdout of one process to stdin of next

---

## 14. Reference Implementation Notes (C#)

### 14.1 Architecture
- **ShaellLexer** + **ShaellParser**: ANTLR4-generated
- **ExecutionVisitor**: extends ShaellParserBaseVisitor<IValue>
  - Recursively visits parse tree
  - Returns IValue from each visit
  - SafeVisit wrapper for error handling with stack traces
- **ScopeManager**: Stack of ScopeContext
  - O(n) lookup through scope stack
  - Simple to implement, easy to copy for closures
- **ScopeContext**: Dictionary<string, RefValue>
  - GetValue() returns null if not found
  - NewValue() creates RefValue, throws if already declared in this scope

### 14.2 Type System Classes
- **IValue** interface: ToBool(), ToNumber(), ToSString(), ToFunction(), ToTable(), ToFile(), Serialize(), IsEqual(), GetTypeName(), Unpack()
- **ITable** interface: GetValue(key), GetKeys()
- **IFunction** interface: Call(args)
- **BaseValue**: base class with default conversions
- **Concrete types**: Number, SString, BString, SBool, SNull, SFile, UserFunc, NativeFunc, UserTable, BaseTable
- **RefValue**: wrapper for mutable references (lvalues)

### 14.3 UserFunc Implementation
- Stores: FunctionBodyContext, captured ScopeManager, formal arguments list, name
- Call(): copies captured scope, pushes new scope, binds args, creates new ExecutionVisitor, visits function body

### 14.4 Piping Implementation
- Uses C# Process class for cross-platform process execution
- ProgramPipelineElement wraps Process
- RunProgProgram checks for INTO (pipe) token, runs process, pipes output to next
- PipeIntoPipeTarget handles recursive piping
- Advanced piping extends simple piping for multiple output streams

---

## 15. Key Differences from Bash
| Feature | Bash | Shæll |
|---------|------|-------|
| Variable declaration | `var=value` | `let var = value` |
| Variable reference | `$var` | `var` |
| Types | Everything is string | Real types (Number, String, Bool, etc.) |
| Math | `$((5 + 5))` | `5 + 5` |
| Decimals | Need `bc` program | Native |
| Functions | `func() { ... }` | `fn func(args) ... end` |
| Function args | `$1, $2, ...` | Named parameters |
| If statement | `if ...; then ... fi` | `if ... then ... end` |
| Piping | `cmd1 | cmd2` | `exec cmd1 -> exec cmd2` |
| File ops | Redirection operators | File object methods |
| Whitespace | Significant in assignments | Not significant |
| String interp | `"$var"` | `"${expr}"` |
