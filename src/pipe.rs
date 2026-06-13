use std::io::Write;
use std::process::{Command, Stdio};

use crate::ast::*;
use crate::error::RuntimeError;
use crate::gc::GcHeap;
use crate::value::*;

/// Execute a ProgProgram (exec ... with (args) -> ...)
pub fn execute_prog_program(
    _heap: &GcHeap,
    prog: &ProgProgram,
    piped_input: Option<String>,
) -> Result<(i32, Option<String>), RuntimeError> {
    // Get the program path
    let program_path = resolve_program_path(&prog.program)?;

    // Evaluate arguments
    let mut args: Vec<String> = Vec::new();
    for arg in &prog.args {
        args.push(expr_to_string(arg)?);
    }

    // Check if we need to capture output (pipe to something)
    let capture_output = prog.pipe_target.is_some() || prog.pipe_expr.is_some();

    // Run the program
    let mut cmd = Command::new(&program_path);
    cmd.args(&args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(if capture_output {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    cmd.stderr(Stdio::inherit());

    let mut child = cmd
        .spawn()
        .map_err(|e| RuntimeError::new(format!("Failed to execute '{}': {}", program_path, e)))?;

    // Write piped input if any
    if let Some(input) = piped_input {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).ok();
        }
    }

    // Wait for the process
    let output = child
        .wait_with_output()
        .map_err(|e| RuntimeError::new(format!("Failed to wait for '{}': {}", program_path, e)))?;

    let return_code = output.status.code().unwrap_or(1);
    let stdout_str = if capture_output {
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        None
    };

    Ok((return_code, stdout_str))
}

/// Execute a PipeProgram (advanced piping)
pub fn execute_pipe_program(heap: &GcHeap, prog: &PipeProgram) -> Result<ValueRef, RuntimeError> {
    let program_path = resolve_program_path(&prog.program)?;

    let mut args: Vec<String> = Vec::new();
    for arg in &prog.args {
        args.push(expr_to_string(arg)?);
    }

    // Collect which streams to capture
    let mut capture_out = false;
    let mut capture_err = false;

    for desc in &prog.descriptors {
        match desc {
            PipeDesc::IntoPipe(stream, _) | PipeDesc::OntoExpr(stream, _) => {
                match stream.as_str() {
                    "out" => capture_out = true,
                    "err" => capture_err = true,
                    _ => {}
                }
            }
        }
    }

    // Run the program
    let mut cmd = Command::new(&program_path);
    cmd.args(&args);
    cmd.stdin(Stdio::null());
    cmd.stdout(if capture_out {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });
    cmd.stderr(if capture_err {
        Stdio::piped()
    } else {
        Stdio::inherit()
    });

    let output = cmd
        .output()
        .map_err(|e| RuntimeError::new(format!("Failed to execute '{}': {}", program_path, e)))?;

    let status_code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Process pipe descriptors sequentially (depth-first)
    // For now, return a result table
    let result_table = heap.allocate(Value::table());
    {
        let status_val = heap.allocate(Value::Number(status_code as f64));
        let out_val = heap.allocate(Value::String(stdout));
        let err_val = heap.allocate(Value::String(stderr));
        heap.with_mut(result_table, |t| {
            t.set_index(&Value::String("status".to_string()), status_val)?;
            t.set_index(&Value::String("out".to_string()), out_val)?;
            t.set_index(&Value::String("err".to_string()), err_val)?;
            Ok::<(), RuntimeError>(())
        })?;
    }

    Ok(result_table)
}

/// Resolve a program expression to a file path
pub(crate) fn resolve_program_path(expr: &Expr) -> Result<String, RuntimeError> {
    match expr {
        Expr::Deref(inner) => match &**inner {
            Expr::String_(s) => {
                let mut path = String::new();
                for part in &s.parts {
                    if let StringPart::Text(t) = part {
                        path.push_str(t);
                    }
                }
                Ok(path)
            }
            Expr::Identifier(name) => Ok(name.clone()),
            _ => Err(RuntimeError::new("Invalid file dereference".to_string())),
        },
        Expr::Identifier(name) => {
            // Bare identifier — look up as file path
            Ok(name.clone())
        }
        Expr::String_(s) => {
            let mut path = String::new();
            for part in &s.parts {
                if let StringPart::Text(t) = part {
                    path.push_str(t);
                }
            }
            Ok(path)
        }
        _ => Err(RuntimeError::new(
            "Cannot resolve program path from expression".to_string(),
        )),
    }
}

/// Convert an expression to a string for argument passing
fn expr_to_string(expr: &Expr) -> Result<String, RuntimeError> {
    match expr {
        Expr::String_(s) => {
            let mut result = String::new();
            for part in &s.parts {
                if let StringPart::Text(t) = part {
                    result.push_str(t);
                }
            }
            Ok(result)
        }
        Expr::Identifier(name) => Ok(name.clone()),
        Expr::Number(n) => Ok(n.to_string()),
        _ => Err(RuntimeError::new(
            "Cannot convert expression to string".to_string(),
        )),
    }
}

/// Execute the full simple piping chain.
/// Returns (return_code, final_stdout).
pub fn run_simple_pipeline(
    heap: &GcHeap,
    prog: &ProgProgram,
    piped_input: Option<String>,
) -> Result<(i32, Option<String>), RuntimeError> {
    let (return_code, stdout) = execute_prog_program(heap, prog, piped_input)?;

    // If there's a pipe target, pipe stdout into it
    if let Some(target) = &prog.pipe_target {
        let inner_prog = match target.as_ref() {
            PipeTarget::ProgProgram(p) => p,
            PipeTarget::PipeProgram(_) => {
                return Err(RuntimeError::new(
                    "Mixed simple/advanced piping not yet supported".to_string(),
                ));
            }
        };
        return run_simple_pipeline(heap, inner_prog, stdout);
    }

    Ok((return_code, stdout))
}
