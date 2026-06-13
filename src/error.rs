use std::fmt;
use std::ops::Range;

/// Result type for parsing
pub type ParseResult<T> = Result<T, ParseError>;

/// Result type for runtime evaluation
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Parse errors
#[derive(Debug, Clone)]
pub enum ParseError {
    UnexpectedToken {
        expected: String,
        found: String,
        span: Range<usize>,
    },
    UnexpectedEof {
        expected: String,
    },
    Custom(String),
}

/// Runtime errors with stack trace support
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub traces: Vec<String>,
}

impl RuntimeError {
    pub fn new(message: String) -> Self {
        RuntimeError {
            message,
            traces: Vec::new(),
        }
    }

    pub fn add_trace(&mut self, trace: String) {
        self.traces.push(trace);
    }

    pub fn with_trace(mut self, trace: String) -> Self {
        self.traces.push(trace);
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken {
                expected,
                found,
                span: _,
            } => {
                write!(f, "Expected {} but found {}", expected, found)
            }
            ParseError::UnexpectedEof { expected } => {
                write!(f, "Expected {} but reached end of file", expected)
            }
            ParseError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Error: {}", self.message)?;
        for trace in self.traces.iter().rev() {
            writeln!(f, "  at {}", trace)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}
impl std::error::Error for RuntimeError {}
