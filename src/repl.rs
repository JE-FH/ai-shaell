use std::io::{self, BufRead, Write};

use crate::interpreter::Interpreter;
use crate::parser::Parser;

/// Run the REPL (Read-Eval-Print Loop)
pub fn run_repl() {
    println!("Shæll REPL v0.1.0");
    println!("Type 'exit' or press Ctrl+D to quit.");
    println!();

    let mut interpreter = Interpreter::new();

    // Try to load .shællrc
    load_shaellrc(&mut interpreter);

    let mut stdin = io::stdin();
    let mut lines = String::new();

    loop {
        print!(">>> ");
        io::stdout().flush().ok();

        lines.clear();
        match stdin.lock().read_line(&mut lines) {
            Ok(0) => {
                // EOF
                println!();
                break;
            }
            Ok(_) => {
                let input = lines.trim();
                if input.is_empty() {
                    continue;
                }
                if input == "exit" || input == "quit" {
                    break;
                }

                // Handle multi-line input for blocks
                let source = if requires_more_lines(input) {
                    read_multiline(&mut stdin, input)
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
                        for trace in &err.traces {
                            eprintln!("  at {}", trace);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Input error: {}", e);
                break;
            }
        }
    }
}

/// Check if input requires more lines (unclosed blocks)
fn requires_more_lines(input: &str) -> bool {
    let mut if_count = 0;
    let mut while_count = 0;
    let mut for_count = 0;
    let mut foreach_count = 0;
    let mut fn_count = 0;
    let mut try_count = 0;
    let mut pipe_count = 0;
    let mut define_args_count = 0;
    let mut do_count = 0;
    let mut end_count = 0;

    for word in input.split_whitespace() {
        match word {
            "if" | "then" => if_count += 1,
            "while" => while_count += 1,
            "for" => for_count += 1,
            "foreach" => foreach_count += 1,
            "fn" => fn_count += 1,
            "try" => try_count += 1,
            "pipe" => pipe_count += 1,
            "define_args" => define_args_count += 1,
            "do" => do_count += 1,
            "end" => end_count += 1,
            _ => {}
        }
    }

    let total_open = if_count
        + while_count
        + for_count
        + foreach_count
        + fn_count
        + try_count
        + pipe_count
        + define_args_count
        + do_count;
    total_open > end_count
}

/// Read multi-line input until all blocks are closed
fn read_multiline(stdin: &mut io::Stdin, initial: &str) -> String {
    let mut buffer = initial.to_string();

    loop {
        print!("... ");
        io::stdout().flush().ok();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                buffer.push('\n');
                buffer.push_str(&line);
                if !requires_more_lines(&buffer) {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    buffer
}

/// Execute Shæll source code
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

/// Try to load and execute .shællrc
fn load_shaellrc(interpreter: &mut Interpreter) {
    let rc_path = dirs_fallback();
    if let Some(mut path) = rc_path {
        path.push(".shællrc");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                match execute_source(interpreter, &content) {
                    Ok(_) => println!("Loaded {}", path.display()),
                    Err(e) => {
                        eprintln!("Warning: error in {}: {}", path.display(), e.message)
                    }
                }
            }
        }
    }
}

/// Get the home directory for .shællrc lookup
fn dirs_fallback() -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(std::path::PathBuf::from)
}

/// Execute a Shæll script file
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
