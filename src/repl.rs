use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

use crate::interpreter::Interpreter;
use crate::parser::Parser;

/// Run the REPL (Read-Eval-Print Loop) with line editing and history.
pub fn run_repl() {
    println!("Shæll v0.2.2 — type 'exit' or Ctrl+D to quit");
    println!();

    let mut interpreter = Interpreter::new();

    // Load ~/.shællrc
    load_shaellrc(&mut interpreter);

    // Set up history
    let mut rl = DefaultEditor::new().expect("failed to create line editor");
    let history_path = history_file();
    let _ = rl.load_history(&history_path);

    loop {
        // Check if we need multi-line input
        let prompt = if requires_more_lines("") {
            "..... "
        } else {
            "shaell$ "
        };

        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }

                rl.add_history_entry(&line).ok();

                // Collect multi-line input if needed
                let source = if requires_more_lines(input) {
                    match read_multiline(&mut rl, input) {
                        Some(multi) => multi,
                        None => continue, // interrupted
                    }
                } else {
                    input.to_string()
                };

                match execute_source(&mut interpreter, &source) {
                    Ok(value) => {
                        let output = interpreter.heap.with_ref(value, |v| v.to_string());
                        if output != "null" {
                            println!("{}", output);
                        }
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err.message);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: cancel current input
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D: exit
                println!("exit");
                break;
            }
            Err(err) => {
                eprintln!("REPL error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);
}

/// Read multi-line input until all blocks are closed.
fn read_multiline(rl: &mut DefaultEditor, initial: &str) -> Option<String> {
    let mut buffer = initial.to_string();

    loop {
        match rl.readline("..... ") {
            Ok(line) => {
                rl.add_history_entry(&line).ok();
                buffer.push('\n');
                buffer.push_str(&line);
                if !requires_more_lines(&buffer) {
                    return Some(buffer);
                }
            }
            Err(ReadlineError::Interrupted) => {
                return None;
            }
            Err(ReadlineError::Eof) => {
                return None;
            }
            Err(_) => return None,
        }
    }
}

/// Check if input requires more lines (unclosed blocks).
fn requires_more_lines(input: &str) -> bool {
    let mut opens = 0i32;
    let mut ends = 0i32;

    for word in input.split_whitespace() {
        match word {
            "if" | "for" | "foreach" | "while" | "fn" | "try" | "pipe" | "define_args" | "then"
            | "do" => opens += 1,
            "end" => ends += 1,
            _ => {}
        }
    }

    // Also check for unclosed strings
    let quote_count = input.matches('"').count();
    let interpolations = input.matches("${").count();
    let close_braces = input.matches('}').count();

    let unbalanced_quotes = !quote_count.is_multiple_of(2);
    let unbalanced_interp = interpolations > close_braces;

    opens > ends || unbalanced_quotes || unbalanced_interp
}

/// Execute Shæll source code.
pub fn execute_source(
    interpreter: &mut Interpreter,
    source: &str,
) -> Result<crate::value::ValueRef, crate::error::RuntimeError> {
    let mut parser = Parser::new(source);
    let program = parser
        .parse_program()
        .map_err(|e| crate::error::RuntimeError::new(format!("Parse error: {}", e)))?;
    interpreter.execute_program(&program)
}

/// Try to load and execute ~/.shællrc.
fn load_shaellrc(interpreter: &mut Interpreter) {
    let rc_path = home_dir().join(".shællrc");
    if rc_path.exists() {
        match std::fs::read_to_string(&rc_path) {
            Ok(content) => match execute_source(interpreter, &content) {
                Ok(_) => eprintln!("Loaded {}", rc_path.display()),
                Err(e) => eprintln!("Warning: error in {}: {}", rc_path.display(), e.message),
            },
            Err(e) => eprintln!("Warning: cannot read {}: {}", rc_path.display(), e),
        }
    }
}

/// Get the home directory.
fn home_dir() -> std::path::PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Path to the history file.
fn history_file() -> std::path::PathBuf {
    home_dir().join(".shæll_history")
}

/// Execute a Shæll script file.
pub fn execute_file(filepath: &str, _args: &[String]) -> Result<(), String> {
    let content = std::fs::read_to_string(filepath)
        .map_err(|e| format!("Cannot read file '{}': {}", filepath, e))?;

    let mut interpreter = Interpreter::new();
    load_shaellrc(&mut interpreter);

    match execute_source(&mut interpreter, &content) {
        Ok(value) => {
            let output = interpreter.heap.with_ref(value, |v| v.to_string());
            if output != "null" {
                println!("{}", output);
            }
            Ok(())
        }
        Err(err) => Err(format!("{}", err)),
    }
}
