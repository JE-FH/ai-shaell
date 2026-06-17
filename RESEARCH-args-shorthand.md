# Research: Shorter argument syntax for `!command`

Currently:
```
!ls with ("-la", "/tmp")
!echo with ("hello")
```

Bash: `ls -la /tmp` (bare words, zero ceremony).

## Approaches considered

### A. Greedy bare-word consumption after `!`

Parser enters "command mode" after `!command` and greedily consumes tokens as string arguments until `->` or end-of-statement.

```
!ls -la /tmp
!echo hello world
!grep -n pattern file.txt -> !wc -l
```

| Token sequence | Becomes |
|---|---|
| `!`, `ls`, `-`, `la`, `/`, `tmp` | `exec ls with ("-la", "/tmp")` |
| `!`, `echo`, `hello`, `world` | `exec echo with ("hello", "world")` |
| `!`, `grep`, `-`, `n`, `pattern`, `file`, `.`, `txt`, `->`, `!`, `wc`, `-`, `l` | `exec grep with ("-n", "pattern", "file.txt") -> exec wc with ("-l")` |

**How it works**: After parsing the command name, enter a loop that peeks at each token. If it's a delimiter (`->`, `then`, `end`, `do`, `else`, `in`, `\n`), stop. Otherwise, consume tokens greedily — identifiers, numbers, operators like `-`, `/`, `.` — and concatenate their text into string arguments, splitting on whitespace tokens.

**Parser change**: ~40 lines in `parse_statement`'s `!` handler. No lexer changes needed.

**Pros**:
- Feels like a shell: `!ls -la /tmp`
- Zero extra syntax — just type words after `!`
- `with (...)` still works for explicit evaluation: `!ls with (variable_path)`
- `->` still works for piping

**Cons**:
- Greedy consumption could eat tokens that belong to the next statement
- `-la` tokenizes as `Minus Identifier("la")` — need to rejoin
- Distinction between program name and first arg could be ambiguous
- If user writes `!echo hello` then `let x = 5` on next line, does `let` become an arg to echo? (No — newline/statement boundary handles this)

### B. Parentheses without `with`

```
!ls("-la", "/tmp")
!echo("hello")
```

Just drop `with` and use parens directly after the command.

**Parser change**: After parsing the command expression, if next token is `(`, consume args from inside parens.

**Pros**: Familiar (looks like function call), minimal parser change (~10 lines)
**Cons**: `!ls(...)` could be confused with `!(ls(...))` — logical not of a function call. Context-dependent.

### C. Backtick raw args

```
!ls `-la /tmp`
!echo `hello world`
```

Raw string in backticks, split on whitespace.

**Pros**: Explicit boundary between args and rest of code
**Cons**: Backtick isn't currently in the grammar at all, new token

### D. Keep `with` but drop parens for single-string case

```
!ls with "-la /tmp"
```

Split the string on spaces into multiple args.

**Pros**: Minimal change
**Cons**: Still verbose, escaping spaces becomes tricky

## Recommendation: **Option A — greedy bare-word consumption**

It's the most shell-like and ergonomic. The implementation:

```rust
// In parse_statement, Token::Bang handler, after parsing program:
let mut args: Vec<Box<Expr>> = Vec::new();

if self.check(&Token::With) {
    // explicit args: !cmd with (a, b)
    self.advance();
    self.expect(&Token::LParen)?;
    args = self.parse_inner_arg_list()?;
    self.expect(&Token::RParen)?;
} else {
    // greedy bare-word args: !cmd -la /tmp
    while !self.is_at_effective_end()
        && !matches!(self.peek_token(), Token::Into | Token::Then | Token::End 
            | Token::Do | Token::Else | Token::If | Token::While | Token::For 
            | Token::Foreach | Token::Return | Token::Fn | Token::Let | Token::Bang)
    {
        // Build a string from consecutive non-whitespace tokens
        let mut arg_text = String::new();
        while let Some(t) = self.peek_raw() {
            if t.token == Token::Whitespace {
                self.advance_raw();
                break;
            }
            arg_text.push_str(&self.source[t.span.start..t.span.end]);
            self.advance_raw();
        }
        if !arg_text.is_empty() {
            args.push(Box::new(Expr::String_(StringLit {
                parts: vec![StringPart::Text(arg_text)],
            })));
        }
    }
}
```

This gives us:
```
!ls -la /tmp
!echo hello world
!grep -n pattern file.txt -> !wc -l
!cat with (computed_path)           # explicit still works
```
