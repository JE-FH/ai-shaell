pub mod ast;
pub mod builtin;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod pipe;
pub mod repl;
pub mod scope;
pub mod token;
pub mod value;

pub mod gc;
#[cfg(test)]
mod tests;
