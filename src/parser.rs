use crate::ast::*;
use crate::error::{ParseError, ParseResult};
use crate::token::{SpannedToken, Token, TokenStream};

/// Recursive descent parser for Shæll
pub struct Parser<'source> {
    tokens: Vec<SpannedToken>,
    pos: usize,
    source: &'source str,
}

impl<'source> Parser<'source> {
    pub fn new(source: &'source str) -> Self {
        let stream = TokenStream::new(source);
        let tokens: Vec<SpannedToken> = stream.collect();
        Parser {
            tokens,
            pos: 0,
            source,
        }
    }

    /// Parse a complete program
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        // Optional program args at the beginning
        let args = if self.check(&Token::DefineArgs) {
            Some(self.parse_program_args()?)
        } else {
            None
        };

        let statements = self.parse_statements_until(&[Token::End])?;

        // Consume any trailing End tokens that close the program
        while self.check(&Token::End) {
            self.advance();
        }

        Ok(Program::new(args, statements))
    }

    // ============================================================
    // Program Arguments
    // ============================================================

    fn parse_program_args(&mut self) -> ParseResult<ProgramArgs> {
        self.expect(&Token::DefineArgs)?;
        let table_name = self.expect_identifier()?;
        let mut arg_names = Vec::new();

        while !self.check(&Token::End) && !self.is_at_end() {
            arg_names.push(self.expect_identifier()?);
            if self.check(&Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::End)?;

        Ok(ProgramArgs {
            table_name,
            arg_names,
        })
    }

    // ============================================================
    // Statements
    // ============================================================

    fn parse_statements_until(&mut self, end_tokens: &[Token]) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.is_at_effective_end() && !end_tokens.iter().any(|t| self.check(t)) {
            if let Some(stmt) = self.parse_statement()? {
                stmts.push(stmt);
            }
        }
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> ParseResult<Option<Stmt>> {
        // Pre-condition: should not be called at end of input
        debug_assert!(
            self.peek().is_some(),
            "parse_statement called at end of input"
        );
        let stmt = match self.peek() {
            Some(t) => match &t.token {
                Token::If => Stmt::If(self.parse_if()?),
                Token::For => Stmt::For(self.parse_for()?),
                Token::While => Stmt::While(self.parse_while()?),
                Token::Foreach => self.parse_foreach()?,
                Token::Return => {
                    self.advance();
                    let expr = self.parse_expression(0)?;
                    Stmt::Return(Box::new(expr))
                }
                Token::Throw => {
                    self.advance();
                    let expr = self.parse_expression(0)?;
                    Stmt::Throw(Box::new(expr))
                }
                Token::Break => {
                    self.advance();
                    Stmt::Break
                }
                Token::Fn => Stmt::FunctionDef(self.parse_function_def()?),
                Token::Pipe => Stmt::PipeProgram(self.parse_pipe_program()?),
                Token::Bang => {
                    self.advance(); // consume !
                                    // Parse only the program name (prefix), not a full expression.
                                    // This prevents `-n` from being parsed as subtraction with the command.
                    let program = self.parse_prefix()?;
                    let program = self.parse_postfix(program)?;
                    let args = if self.check(&Token::With) {
                        self.advance();
                        self.expect(&Token::LParen)?;
                        let args = self.parse_inner_arg_list()?;
                        self.expect(&Token::RParen)?;
                        args
                    } else {
                        self.parse_bare_args()?
                    };
                    let (pipe_target, pipe_expr) = if self.check(&Token::Into) {
                        self.advance();
                        if self.check(&Token::Bang)
                            || self.check(&Token::Exec)
                            || self.check(&Token::Pipe)
                        {
                            (Some(Box::new(self.parse_pipe_target()?)), None)
                        } else {
                            let expr = self.parse_expression(0)?;
                            (None, Some(Box::new(expr)))
                        }
                    } else {
                        (None, None)
                    };
                    Stmt::Expr(Expr::ProgProgram(ProgProgram {
                        program: Box::new(program),
                        args,
                        pipe_target,
                        pipe_expr,
                    }))
                }
                Token::End | Token::RCurl => return Ok(None),
                _ => Stmt::Expr(self.parse_expression(0)?),
            },
            None => return Ok(None),
        };
        Ok(Some(stmt))
    }

    // ============================================================
    // Control Structures
    // ============================================================

    fn parse_if(&mut self) -> ParseResult<IfStmt> {
        self.expect(&Token::If)?;
        let condition = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Then)?;
        let then_body = self.parse_statements_until(&[Token::Else, Token::End])?;

        let else_body = if self.check(&Token::Else) {
            self.advance();
            let body = self.parse_statements_until(&[Token::End])?;
            Some(body)
        } else {
            None
        };

        self.expect(&Token::End)?;

        Ok(IfStmt {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_while(&mut self) -> ParseResult<WhileLoop> {
        self.expect(&Token::While)?;
        let condition = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Do)?;
        let body = self.parse_statements_until(&[Token::End])?;
        self.expect(&Token::End)?;

        Ok(WhileLoop { condition, body })
    }

    fn parse_for(&mut self) -> ParseResult<ForLoop> {
        self.expect(&Token::For)?;
        let init = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Comma)?;
        let condition = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Comma)?;
        let update = Box::new(self.parse_expression(0)?);
        self.expect(&Token::Do)?;
        let body = self.parse_statements_until(&[Token::End])?;
        self.expect(&Token::End)?;

        Ok(ForLoop {
            init,
            condition,
            update,
            body,
        })
    }

    fn parse_foreach(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Foreach)?;

        // Check if key-value foreach: foreach key, value in ...
        let first = self.expect_identifier()?;

        if self.check(&Token::Comma) {
            // key-value foreach
            self.advance();
            let value_var = self.expect_identifier()?;
            self.expect(&Token::In)?;
            let collection = Box::new(self.parse_expression(0)?);
            self.expect(&Token::Do)?;
            let body = self.parse_statements_until(&[Token::End])?;
            self.expect(&Token::End)?;

            Ok(Stmt::ForeachKeyValue(ForeachKeyValueLoop {
                key_var: first,
                value_var,
                collection,
                body,
            }))
        } else {
            // value-only foreach
            self.expect(&Token::In)?;
            let collection = Box::new(self.parse_expression(0)?);
            self.expect(&Token::Do)?;
            let body = self.parse_statements_until(&[Token::End])?;
            self.expect(&Token::End)?;

            Ok(Stmt::Foreach(ForeachLoop {
                var: first,
                collection,
                body,
            }))
        }
    }

    // ============================================================
    // Functions
    // ============================================================

    fn parse_function_def(&mut self) -> ParseResult<FunctionDef> {
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_formal_args()?;
        self.expect(&Token::RParen)?;
        let body = self.parse_function_body()?;

        Ok(FunctionDef { name, params, body })
    }

    fn parse_formal_args(&mut self) -> ParseResult<Vec<String>> {
        let mut params = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                params.push(self.expect_identifier()?);
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        Ok(params)
    }

    fn parse_function_body(&mut self) -> ParseResult<FunctionBody> {
        // Lambda body: fn (x) => expression
        if self.check(&Token::Lambda) {
            self.advance();
            let expr = self.parse_expression(0)?;
            return Ok(FunctionBody::Lambda(Box::new(expr)));
        }

        // Block body: fn (x) stmts end
        let body = self.parse_statements_until(&[Token::End])?;
        self.expect(&Token::End)?;
        Ok(FunctionBody::Block(body))
    }

    // ============================================================
    // Piping
    // ============================================================

    fn parse_prog_program(&mut self) -> ParseResult<ProgProgram> {
        // Accept both exec and ! as program execution prefixes
        let is_bang = self.check(&Token::Bang);
        if self.check(&Token::Exec) || is_bang {
            self.advance();
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "'exec' or '!'".to_string(),
                found: self.peek().map(|t| t.token.to_string()).unwrap_or_default(),
                span: self.peek().map(|t| t.span.clone()).unwrap_or(0..0),
            });
        }
        let program = {
            let p = self.parse_prefix()?;
            Box::new(self.parse_postfix(p)?)
        };

        let args = if self.check(&Token::With) {
            self.advance();
            self.expect(&Token::LParen)?;
            let args = self.parse_inner_arg_list()?;
            self.expect(&Token::RParen)?;
            args
        } else if is_bang {
            self.parse_bare_args()?
        } else {
            Vec::new()
        };

        let (pipe_target, pipe_expr) = if self.check(&Token::Into) {
            self.advance();
            if self.check(&Token::Exec) || self.check(&Token::Bang) || self.check(&Token::Pipe) {
                (Some(Box::new(self.parse_pipe_target()?)), None)
            } else {
                let expr = self.parse_expression(0)?;
                (None, Some(Box::new(expr)))
            }
        } else {
            (None, None)
        };

        Ok(ProgProgram {
            program,
            args,
            pipe_target,
            pipe_expr,
        })
    }

    fn parse_pipe_program(&mut self) -> ParseResult<PipeProgram> {
        self.expect(&Token::Pipe)?;
        let program = {
            let p = self.parse_prefix()?;
            Box::new(self.parse_postfix(p)?)
        };

        let args = if self.check(&Token::With) {
            self.advance();
            self.expect(&Token::LParen)?;
            let args = self.parse_inner_arg_list()?;
            self.expect(&Token::RParen)?;
            args
        } else {
            Vec::new()
        };

        let mut descriptors = Vec::new();
        while !self.check(&Token::End) && !self.is_at_end() {
            descriptors.push(self.parse_pipe_desc()?);
        }
        self.expect(&Token::End)?;

        Ok(PipeProgram {
            program,
            args,
            descriptors,
        })
    }

    fn parse_pipe_target(&mut self) -> ParseResult<PipeTarget> {
        if self.check(&Token::Exec) || self.check(&Token::Bang) {
            Ok(PipeTarget::ProgProgram(self.parse_prog_program()?))
        } else {
            Ok(PipeTarget::PipeProgram(self.parse_pipe_program()?))
        }
    }

    fn parse_pipe_desc(&mut self) -> ParseResult<PipeDesc> {
        let stream_name = self.expect_identifier()?;
        self.expect(&Token::Into)?;

        if self.check(&Token::Exec) || self.check(&Token::Bang) || self.check(&Token::Pipe) {
            Ok(PipeDesc::IntoPipe(stream_name, self.parse_pipe_target()?))
        } else {
            let expr = self.parse_expression(0)?;
            Ok(PipeDesc::OntoExpr(stream_name, Box::new(expr)))
        }
    }

    fn parse_inner_arg_list(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                args.push(self.parse_expression(0)?);
                if !self.check(&Token::Comma) {
                    break;
                }
                self.advance();
            }
        }
        Ok(args)
    }

    /// Parse bare-word arguments after !command.
    /// Greedily consumes tokens as literal string arguments until a
    /// delimiter (->, statement keyword, EOF) is hit.
    /// Tokens are rejoined from source spans to preserve original text.
    /// Parse a bare command + args as an expression (stops at ), ], ,, :, RCurl).
    fn parse_bare_cmd_expr(&mut self) -> ParseResult<ProgProgram> {
        let program = {
            let p = self.parse_prefix()?;
            Box::new(self.parse_postfix(p)?)
        };
        let args = if self.check(&Token::With) {
            self.advance();
            self.expect(&Token::LParen)?;
            let args = self.parse_inner_arg_list()?;
            self.expect(&Token::RParen)?;
            args
        } else {
            self.parse_bare_args_expr()?
        };
        Ok(ProgProgram {
            program,
            args,
            pipe_target: None,
            pipe_expr: None,
        })
    }

    /// Parse bare args in expression context — stops at expression delimiters.
    fn parse_bare_args_expr(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        let delimiters: &[Token] = &[
            Token::Into,
            Token::RParen,
            Token::RSquare,
            Token::Comma,
            Token::Colon,
            Token::RCurl,
            Token::End,
            Token::Then,
            Token::Else,
            Token::Do,
            Token::In,
        ];

        loop {
            // Skip whitespace/semicolons
            while self.pos < self.tokens.len()
                && (self.tokens[self.pos].token == Token::Whitespace
                    || self.tokens[self.pos].token == Token::Semicolon)
            {
                self.pos += 1;
            }
            if self.pos >= self.tokens.len() {
                break;
            }
            if delimiters.contains(&self.tokens[self.pos].token) {
                break;
            }
            // Handle quoted strings
            if self.tokens[self.pos].token == Token::DQuote {
                self.pos += 1;
                let expr = self.parse_string_literal()?;
                args.push(expr);
                continue;
            }
            // Collect consecutive non-whitespace, non-delimiter tokens
            let start = self.tokens[self.pos].span.start;
            while self.pos < self.tokens.len()
                && self.tokens[self.pos].token != Token::Whitespace
                && self.tokens[self.pos].token != Token::Semicolon
                && !delimiters.contains(&self.tokens[self.pos].token)
                && self.tokens[self.pos].token != Token::DQuote
            {
                self.pos += 1;
            }
            let end = self.tokens[self.pos - 1].span.end;
            let text = self.source[start..end].to_string();
            if !text.is_empty() {
                args.push(Expr::String_(StringLit {
                    parts: vec![StringPart::Text(text)],
                }));
            }
        }
        Ok(args)
    }

    /// Parse a bare command + args inside !{cmd args}.
    fn parse_bare_cmd(&mut self) -> ParseResult<ProgProgram> {
        let program = {
            let p = self.parse_prefix()?;
            Box::new(self.parse_postfix(p)?)
        };
        let args = if self.check(&Token::With) {
            self.advance();
            self.expect(&Token::LParen)?;
            let args = self.parse_inner_arg_list()?;
            self.expect(&Token::RParen)?;
            args
        } else {
            self.parse_bare_args()?
        };
        Ok(ProgProgram {
            program,
            args,
            pipe_target: None,
            pipe_expr: None,
        })
    }

    fn parse_bare_args(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        let delimiters: &[Token] = &[
            Token::Into,
            Token::End,
            Token::Then,
            Token::Else,
            Token::Do,
            Token::In,
            Token::If,
            Token::While,
            Token::For,
            Token::Foreach,
            Token::Return,
            Token::Fn,
            Token::Let,
            Token::Pipe,
            Token::Exec,
            Token::Bang,
            Token::RCurl,
        ];

        loop {
            // Skip leading whitespace; track if we crossed a newline
            let mut crossed_newline = false;
            while self.pos < self.tokens.len()
                && (self.tokens[self.pos].token == Token::Whitespace
                    || self.tokens[self.pos].token == Token::Semicolon)
            {
                let span = &self.tokens[self.pos].span;
                if self.source[span.start..span.end].contains('\n') {
                    crossed_newline = true;
                }
                self.pos += 1;
            }

            if self.pos >= self.tokens.len() {
                break;
            }

            // Stop at delimiters or after a newline (statement boundary)
            if delimiters.contains(&self.tokens[self.pos].token) {
                break;
            }
            if crossed_newline {
                break;
            }

            // Handle quoted strings: parse as proper string literal
            if self.tokens[self.pos].token == Token::DQuote {
                // Use the standard string parser
                self.pos += 1; // skip opening DQuote
                let expr = self.parse_string_literal()?;
                args.push(expr);
                continue;
            }

            // Collect consecutive non-whitespace, non-delimiter tokens
            // into a single string argument
            let start = self.tokens[self.pos].span.start;
            while self.pos < self.tokens.len()
                && self.tokens[self.pos].token != Token::Whitespace
                && !delimiters.contains(&self.tokens[self.pos].token)
                && self.tokens[self.pos].token != Token::DQuote
            {
                self.pos += 1;
            }
            let end = self.tokens[self.pos - 1].span.end;
            let text = self.source[start..end].to_string();

            if !text.is_empty() {
                args.push(Expr::String_(StringLit {
                    parts: vec![StringPart::Text(text)],
                }));
            }
        }

        // Post-condition: all args should be non-empty string literals
        debug_assert!(
            args.iter()
                .all(|arg| matches!(arg, Expr::String_(sl) if !sl.parts.is_empty())),
            "parse_bare_args: args should all be non-empty string literals"
        );
        Ok(args)
    }

    // ============================================================
    // Expressions — Precedence Climbing
    // ============================================================

    /// Parse an expression with precedence climbing.
    /// `min_prec` is the minimum precedence level to parse.
    pub fn parse_expression(&mut self, min_prec: u8) -> ParseResult<Expr> {
        // Pre-condition: min_prec must be within the valid precedence range (0..=8)
        debug_assert!(
            min_prec <= 8,
            "parse_expression: min_prec {} exceeds max precedence 8",
            min_prec
        );
        // Handle prefix atom
        let mut left = self.parse_prefix()?;

        // Apply postfix operations (colon, subscript, call) — highest precedence
        left = self.parse_postfix(left)?;

        // Infix operators with precedence climbing
        while let Some(t) = self.peek() {
            let token = t.token.clone();

            // Skip token terminators — these are not infix operators
            if matches!(
                &token,
                Token::End | Token::Then | Token::Do | Token::RCurl | Token::Else | Token::In
            ) {
                break;
            }

            let prec = get_infix_precedence(&token);
            // Only handle tokens that are actual infix operators (precedence > 0)
            if prec == 0 || prec < min_prec {
                break;
            }

            // Handle right-associative operators
            let next_min_prec = if is_right_associative(&token) {
                prec
            } else {
                prec + 1
            };

            match &token {
                Token::Into => {
                    // Into is part of progProgram pipe syntax, not a standalone binary op
                    break;
                }
                _ => {
                    self.advance();
                    let right = self.parse_expression(next_min_prec)?;
                    let right = self.parse_postfix(right)?;
                    left = self.make_binary_or_assign_expr(&token, left, right);
                }
            }
        }

        Ok(left)
    }

    fn make_binary_or_assign_expr(&self, token: &Token, left: Expr, right: Expr) -> Expr {
        match token {
            Token::Assign => Expr::Assign(Box::new(left), Box::new(right)),
            Token::PlusEq => Expr::PlusEq(Box::new(left), Box::new(right)),
            Token::MinusEq => Expr::MinusEq(Box::new(left), Box::new(right)),
            Token::MultEq => Expr::MultEq(Box::new(left), Box::new(right)),
            Token::DivEq => Expr::DivEq(Box::new(left), Box::new(right)),
            Token::ModEq => Expr::ModEq(Box::new(left), Box::new(right)),
            Token::PowEq => Expr::PowEq(Box::new(left), Box::new(right)),
            _ => self.make_binary_expr(token, left, right),
        }
    }

    fn make_binary_expr(&self, token: &Token, left: Expr, right: Expr) -> Expr {
        match token {
            Token::Pow => Expr::Pow(Box::new(left), Box::new(right)),
            Token::Mult => Expr::Mult(Box::new(left), Box::new(right)),
            Token::Div => Expr::Div(Box::new(left), Box::new(right)),
            Token::Mod => Expr::Mod(Box::new(left), Box::new(right)),
            Token::Plus => Expr::Add(Box::new(left), Box::new(right)),
            Token::Minus => Expr::Sub(Box::new(left), Box::new(right)),
            Token::LT => Expr::LT(Box::new(left), Box::new(right)),
            Token::LEQ => Expr::LEQ(Box::new(left), Box::new(right)),
            Token::GT => Expr::GT(Box::new(left), Box::new(right)),
            Token::GEQ => Expr::GEQ(Box::new(left), Box::new(right)),
            Token::EQ => Expr::EQ(Box::new(left), Box::new(right)),
            Token::NEQ => Expr::NEQ(Box::new(left), Box::new(right)),
            Token::LAnd => Expr::LAnd(Box::new(left), Box::new(right)),
            Token::LOr => Expr::LOr(Box::new(left), Box::new(right)),
            _ => unreachable!("Unexpected binary operator: {:?}", token),
        }
    }

    // ============================================================
    // Prefix (Atomic) Expressions
    // ============================================================

    fn parse_prefix(&mut self) -> ParseResult<Expr> {
        let token = self.advance().clone();

        match &token.token {
            // Literals
            Token::Number(n) => Ok(Expr::Number(*n)),
            Token::True => Ok(Expr::Boolean(true)),
            Token::False => Ok(Expr::Boolean(false)),
            Token::Null => Ok(Expr::Null),
            Token::DQuote => self.parse_string_literal(),

            // Identifier
            Token::Identifier(name) => Ok(Expr::Identifier(name.clone())),

            // Let declaration
            Token::Let => {
                let name = self.expect_identifier()?;
                if self.check(&Token::Assign) {
                    self.advance();
                    let expr = self.parse_expression(0)?;
                    Ok(Expr::LetAssign(name, Box::new(expr)))
                } else {
                    Ok(Expr::Let(name))
                }
            }

            // Unary prefix operators
            Token::Minus => {
                let expr = self.parse_prefix()?;
                let expr = self.parse_postfix(expr)?;
                Ok(Expr::UnaryNeg(Box::new(expr)))
            }
            Token::Plus => {
                let expr = self.parse_prefix()?;
                let expr = self.parse_postfix(expr)?;
                Ok(Expr::UnaryPlus(Box::new(expr)))
            }
            Token::Not => {
                let expr = self.parse_prefix()?;
                let expr = self.parse_postfix(expr)?;
                Ok(Expr::UnaryNot(Box::new(expr)))
            }
            Token::Bang => {
                // ! always means exec, even in expression context.
                // !{cmd args} → explicit inline command substitution
                // !cmd args   → bare inline (stops at ), ], ,, :, end-of-expr)
                if self.pos < self.tokens.len() && self.tokens[self.pos].token == Token::LCurl {
                    self.advance_raw(); // consume {
                    let cmd = self.parse_bare_cmd()?;
                    self.expect(&Token::RCurl)?;
                    Ok(Expr::CmdSub(cmd))
                } else {
                    // Bare !cmd args — parse command + args, stop at expression delimiters
                    let cmd = self.parse_bare_cmd_expr()?;
                    Ok(Expr::CmdSub(cmd))
                }
            }
            Token::Deref => {
                let expr = self.parse_prefix()?;
                let expr = self.parse_postfix(expr)?;
                Ok(Expr::Deref(Box::new(expr)))
            }

            // Grouping
            Token::LParen => {
                let expr = self.parse_expression(0)?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }

            // Table/Object literal
            Token::LCurl => self.parse_object_literal(),

            // Try expression
            Token::Try => {
                let stmts = self.parse_statements_until(&[Token::End])?;
                self.expect(&Token::End)?;
                Ok(Expr::Try(stmts))
            }

            // External program execution
            Token::Exec => {
                // We already consumed 'exec', need to re-parse from that point
                // The simplest approach: un-advance and call parse_prog_program
                // Since parse_prog_program expects exec first, we need a variant.
                // Actually, just build the ProgProgram directly from here.
                let program = self.parse_expression(0)?;

                let args = if self.check(&Token::With) {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    let args = self.parse_inner_arg_list()?;
                    self.expect(&Token::RParen)?;
                    args
                } else {
                    Vec::new()
                };

                let (pipe_target, pipe_expr) = if self.check(&Token::Into) {
                    self.advance();
                    if self.check(&Token::Exec)
                        || self.check(&Token::Bang)
                        || self.check(&Token::Pipe)
                    {
                        (Some(Box::new(self.parse_pipe_target()?)), None)
                    } else {
                        let expr = self.parse_expression(0)?;
                        (None, Some(Box::new(expr)))
                    }
                } else {
                    (None, None)
                };

                Ok(Expr::ProgProgram(ProgProgram {
                    program: Box::new(program),
                    args,
                    pipe_target,
                    pipe_expr,
                }))
            }

            // Anonymous function (fn already consumed by advance())
            Token::Fn => {
                self.expect(&Token::LParen)?;
                let params = self.parse_formal_args()?;
                self.expect(&Token::RParen)?;
                let body = self.parse_function_body()?;
                Ok(Expr::AnonFnDef(AnonFunctionDef { params, body }))
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: token.token.to_string(),
                span: token.span.clone(),
            }),
        }
    }

    // ============================================================
    // String Literal Parsing
    // ============================================================

    fn parse_string_literal(&mut self) -> ParseResult<Expr> {
        let mut parts = Vec::new();

        loop {
            if self.is_at_end() {
                return Err(ParseError::UnexpectedEof {
                    expected: "\" or string content".to_string(),
                });
            }

            let token = self.tokens[self.pos].token.clone();
            let _span = self.tokens[self.pos].span.clone();

            match &token {
                Token::Whitespace => {
                    // Whitespace inside strings is literal text
                    self.advance_raw();
                    parts.push(StringPart::Text(" ".to_string()));
                }
                Token::DQuote => {
                    self.advance_raw();
                    break;
                }
                Token::Interpolation => {
                    self.advance_raw();
                    let expr = self.parse_expression(0)?;
                    // After parsing expression, the next non-whitespace should be RCurl
                    if self.peek().map(|t| &t.token) == Some(&Token::RCurl) {
                        self.advance();
                    } else {
                        let found = self
                            .peek()
                            .map(|t| t.token.to_string())
                            .unwrap_or_else(|| "EOF".to_string());
                        return Err(ParseError::UnexpectedToken {
                            expected: "'}'".to_string(),
                            found,
                            span: _span,
                        });
                    }
                    parts.push(StringPart::Interpolation(Box::new(expr)));
                }
                Token::EscapedNewline => {
                    self.advance_raw();
                    parts.push(StringPart::Newline);
                }
                Token::EscapedDollar => {
                    self.advance_raw();
                    parts.push(StringPart::EscapedDollar);
                }
                Token::EscapedBackslash => {
                    self.advance_raw();
                    parts.push(StringPart::EscapedBackslash);
                }
                Token::Identifier(s) => {
                    let s = s.clone();
                    self.advance_raw();
                    parts.push(StringPart::Text(s));
                }
                Token::Number(n) => {
                    let s = n.to_string();
                    self.advance_raw();
                    parts.push(StringPart::Text(s));
                }
                _ => {
                    // Use raw source text from the token span
                    let span = self.tokens[self.pos].span.clone();
                    let text = &self.source[span.start..span.end];
                    self.advance_raw();
                    parts.push(StringPart::Text(text.to_string()));
                }
            }
        }

        Ok(Expr::String_(StringLit { parts }))
    }

    // ============================================================
    // Object/Table Literal
    // ============================================================

    fn parse_object_literal(&mut self) -> ParseResult<Expr> {
        // LCurl already consumed
        let mut fields = Vec::new();

        if self.check(&Token::RCurl) {
            self.advance();
            return Ok(Expr::Object(fields));
        }

        loop {
            let field = if self.check(&Token::LSquare) {
                // [expr] = value
                self.advance();
                let index = self.parse_expression(0)?;
                self.expect(&Token::RSquare)?;
                ObjectField::Subscript(Box::new(index))
            } else {
                // identifier = value
                let name = self.expect_identifier()?;
                ObjectField::Identifier(name)
            };

            self.expect(&Token::Assign)?;
            let value = self.parse_expression(0)?;
            fields.push((field, Box::new(value)));

            if self.check(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.expect(&Token::RCurl)?;
        Ok(Expr::Object(fields))
    }

    // ============================================================
    // Postfix Operations (called after prefix)
    // ============================================================

    fn parse_postfix(&mut self, mut expr: Expr) -> ParseResult<Expr> {
        while let Some(t) = self.peek() {
            let token = t.token.clone();

            match &token {
                Token::Colon => {
                    self.advance();
                    let key = self.expect_identifier()?;
                    expr = Expr::IndexColon(Box::new(expr), key);
                }
                Token::LSquare => {
                    self.advance();
                    let index = self.parse_expression(0)?;
                    self.expect(&Token::RSquare)?;
                    expr = Expr::IndexSubscript(Box::new(expr), Box::new(index));
                }
                Token::LParen => {
                    self.advance();
                    let args = self.parse_inner_arg_list()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    // ============================================================
    // Helper Methods
    // ============================================================

    fn peek(&self) -> Option<&SpannedToken> {
        // Skip whitespace and semicolon tokens
        let mut pos = self.pos;
        let result = loop {
            if pos >= self.tokens.len() {
                break None;
            }
            let t = &self.tokens[pos].token;
            if t != &Token::Whitespace && t != &Token::Semicolon {
                break Some(&self.tokens[pos]);
            }
            pos += 1;
        };
        // Post-condition: if Some, the returned token is never whitespace/semicolon;
        // if None, all remaining tokens are whitespace/semicolon or stream is exhausted
        debug_assert!(
            match result {
                Some(t) => t.token != Token::Whitespace && t.token != Token::Semicolon,
                None => (self.pos..self.tokens.len()).all(|i| {
                    let t = &self.tokens[i].token;
                    t == &Token::Whitespace || t == &Token::Semicolon
                }),
            },
            "peek post-condition violated"
        );
        result
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek().map(|t| &t.token == expected).unwrap_or(false)
    }

    fn advance(&mut self) -> &SpannedToken {
        // Skip whitespace and semicolon tokens
        while self.pos < self.tokens.len()
            && (self.tokens[self.pos].token == Token::Whitespace
                || self.tokens[self.pos].token == Token::Semicolon)
        {
            self.pos += 1;
        }
        if self.pos < self.tokens.len() {
            let idx = self.pos;
            self.pos += 1;
            &self.tokens[idx]
        } else {
            panic!("Parser advanced past end of token stream");
        }
    }

    fn advance_raw(&mut self) -> &SpannedToken {
        if self.pos < self.tokens.len() {
            let idx = self.pos;
            self.pos += 1;
            &self.tokens[idx]
        } else {
            panic!("Parser advanced past end of token stream");
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn is_at_effective_end(&self) -> bool {
        let mut pos = self.pos;
        while pos < self.tokens.len()
            && (self.tokens[pos].token == Token::Whitespace
                || self.tokens[pos].token == Token::Semicolon)
        {
            pos += 1;
        }
        pos >= self.tokens.len()
    }

    fn expect(&mut self, expected: &Token) -> ParseResult<&SpannedToken> {
        // Pre-condition: current position is within valid bounds
        debug_assert!(
            self.pos <= self.tokens.len(),
            "expect({:?}): position {} is past end (len {})",
            expected,
            self.pos,
            self.tokens.len()
        );
        let pos_before = self.pos;
        if self.check(expected) {
            self.advance();
            // Post-condition: position advanced after consuming the token
            debug_assert!(
                self.pos > pos_before,
                "expect({:?}) did not advance position",
                expected
            );
            Ok(&self.tokens[self.pos - 1])
        } else {
            let found = self
                .peek()
                .map(|t| t.token.to_string())
                .unwrap_or_else(|| "EOF".to_string());
            let span = self.peek().map(|t| t.span.clone()).unwrap_or(0..0);
            Err(ParseError::UnexpectedToken {
                expected: expected.to_string(),
                found,
                span,
            })
        }
    }

    fn expect_identifier(&mut self) -> ParseResult<String> {
        match self.peek() {
            Some(t) => match &t.token {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    Ok(name)
                }
                _ => Err(ParseError::UnexpectedToken {
                    expected: "identifier".to_string(),
                    found: t.token.to_string(),
                    span: t.span.clone(),
                }),
            },
            None => Err(ParseError::UnexpectedEof {
                expected: "identifier".to_string(),
            }),
        }
    }
}

// ============================================================
// Operator Precedence Helpers
// ============================================================

/// Get the precedence level for infix operators.
/// Higher number = higher precedence (binds tighter).
/// Returns 0 for tokens that are not infix operators.
fn get_infix_precedence(token: &Token) -> u8 {
    match token {
        // Assignment (lowest precedence among real infix operators)
        Token::Assign => 1,
        // Compound assignment
        Token::PlusEq
        | Token::MinusEq
        | Token::MultEq
        | Token::DivEq
        | Token::ModEq
        | Token::PowEq => 1,
        // Logical or
        Token::LOr => 2,
        // Logical and
        Token::LAnd => 3,
        // Equality
        Token::EQ | Token::NEQ => 4,
        // Comparison
        Token::LT | Token::LEQ | Token::GT | Token::GEQ => 5,
        // Addition/subtraction
        Token::Plus | Token::Minus => 6,
        // Multiplication/division/modulo
        Token::Mult | Token::Div | Token::Mod => 7,
        // Power
        Token::Pow => 8,
        _ => 0,
    }
}

/// Check if an operator is right-associative
fn is_right_associative(token: &Token) -> bool {
    matches!(
        token,
        Token::Assign
            | Token::PlusEq
            | Token::MinusEq
            | Token::MultEq
            | Token::DivEq
            | Token::ModEq
            | Token::PowEq
            | Token::Pow
    )
}
