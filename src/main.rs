use std::env;

use shaell::repl;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 1 {
        // No arguments — start REPL
        repl::run_repl();
    } else {
        // Execute a script file
        let filepath = &args[1];
        let script_args = &args[2..];

        match repl::execute_file(filepath, script_args) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
    }
}
