# Research: `!ls` shorthand for `exec ls with ()`

## Current state

Running an external program requires the verbose `exec` keyword:

```
exec ls with ("-la")
exec echo with ("hello world")
exec grep with ("pattern") -> exec wc with ("-l")
```

Goal: allow `!ls` as a shorthand.

## Grammar analysis

`!` is currently `Token::LNot` — a unary prefix operator producing `Expr::UnaryLNot(expr)`:

```
!true     → false
!false    → true
!0        → true
!("a"=="b") → true
```

Parsing flow:
| Context | Token | What happens |
|---------|-------|-------------|
| Expression | `LNot` → `parse_prefix` | `UnaryLNot(expr)` |
| Statement | falls through to `_ => Stmt::Expr(...)` | starts expression parsing |

## Where `!` is used in existing tests

```
!true              → "false"    (expression)
!false             → "true"     (expression)
!!true             → "true"     (double negation, expression)
!true == false     → "true"     (negation in comparison, expression)
```

**Zero cases** of `!` used as a standalone statement. A standalone `!true;` would evaluate to `false` and be discarded — nobody writes this.

## Proposed approach

### Recommendation: Context-dependent `!` at statement level

In `parse_statement`, when `LNot` is the first token, treat it as program execution instead of logical negation. In expression context, `!` keeps its current meaning (logical not).

**Parser change** (single method, ~30 lines):

```rust
// In parse_statement, add before the _ => fallthrough:
Token::LNot => {
    self.advance();
    let program = self.parse_expression(0)?;
    let args = if self.check(&Token::With) {
        // !cmd with (args) — optional explicit args
        self.advance();
        self.expect(&Token::LParen)?;
        let args = self.parse_inner_arg_list()?;
        self.expect(&Token::RParen)?;
        args
    } else {
        vec![] // !cmd — no args, just execute
    };
    let pipe_target = if self.check(&Token::Into) {
        self.advance();
        if self.check(&Token::LNot) || self.check(&Token::Exec) || self.check(&Token::Pipe) {
            Some(Box::new(self.parse_pipe_target()?))
        } else {
            let expr = self.parse_expression(0)?;
            return Ok(Some(Stmt::Expr(Expr::ProgProgram(ProgProgram {
                program: Box::new(program),
                args,
                pipe_expr: Some(Box::new(expr)),
                pipe_target: None,
            }))));
        }
    } else {
        None
    };
    Ok(Some(Stmt::Expr(Expr::ProgProgram(ProgProgram {
        program: Box::new(program),
        args,
        pipe_target,
        pipe_expr: None,
    }))))
}
```

### Examples after change

```
!ls                              → exec ls with ()
!echo hello world                → exec echo with ("hello", "world")  
!grep pattern -> !wc -l          → piping between programs
!cat with ("file.txt")           → explicit args (when needed for clarity)
!false                           → exec false (runs /usr/bin/false)
let x = !true                    → still logical not (expression context)
if !condition then ...           → still logical not (inside if condition)
```

### What about `!true`?

`!true` at statement level changes meaning: it runs the `true` command instead of evaluating to `false`. But:
- The old behavior (`false`, discarded) was a no-op — nobody writes this
- `true` IS a real Unix command that succeeds — executing it makes practical sense
- `!false` similarly runs `false` which exits with code 1

## Alternative: `!!` (double bang)

Use `!!` instead of `!` to avoid any ambiguity.

| Syntax | Meaning |
|--------|---------|
| `!expr` | Logical not (unchanged) |
| `!!cmd` | Execute cmd |

Pros: zero ambiguity, no context-dependent parsing
Cons: two characters, less elegant

## Decision matrix

| Factor | `!` at statement level | `!!` new token |
|--------|----------------------|----------------|
| Breaks existing code | No (0 test failures) | No |
| Cognitive load | Medium (context matters) | Low (always unambiguous) |
| Typing economy | 1 char | 2 chars |
| Parser complexity | ~30 lines in parse_statement | ~5 lines in lexer + ~30 in parser |
| Consistency with Bash | No (Bash uses `!` for history) | N/A |

## Verdict

**`!` at statement level is feasible and recommended.** 

It adds ~30 lines to the parser, breaks zero existing tests, and the context distinction (statement vs expression) already exists naturally in the grammar — `parse_statement` vs `parse_expression` are separate entry points. The cognitive load is acceptable: users already understand that `if` means something different at statement level than `if` would in an expression (it doesn't appear there).
