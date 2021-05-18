use super::*;

type ExprResult = Result<Expr, Error>;

/// Generate infix expression by pattern left <op> right
///
/// Credits: `@! ! Reiter#4543`
#[macro_export]
macro_rules! gen_infix {
    ($($fn_name: ident => $type: tt);*$(;)?) => {$(
        /// Generated function for infix operator
        fn $fn_name(&mut self, left: Expr) -> ExprResult {
            self.lexer.next();
            let next = self.lexer.next();
            let right = self.parse_expr(next)?;
            Ok(Expr::$type { left: Box::new(left), right: Box::new(right) })
        })*
    };
}

impl<'a> Parser<'a> {
    pub(super) fn parse_ops(&mut self, token: SpannedToken) -> ParseResult {
        // Statements
        match self.lexer.peek() {
            Some(Token::LeftParenthesis) => return self.fn_call(token),
            Some(Token::Assignment) => return self.parse_assignment(token),
            _ => (),
        }

        let mut buf: Expr = self.parse_expr(Some(token))?;

        loop {
            let peek = self.lexer.peek().clone();
            buf = match peek {
                // Print statement
                Some(Token::Print) => {
                    self.lexer.next();
                    self.require(Token::Semicolon)?;
                    return Ok(Stmt::Print(buf).into());
                }
                None => return Ok(buf.into()),

                // An expression
                _ => self.parse_operation(peek, buf)?,
            }
        }
    }

    /// Match and perform
    pub(super) fn parse_operation(&mut self, token: Option<SpannedToken>, buf: Expr) -> ExprResult {
        match token {
            Some((Token::Addition, span)) => self.addition(buf),
            Some((Token::Subtract, span)) => self.subtract(buf),
            Some((Token::Multiply, span)) => self.multiply(buf),
            Some((Token::Divide, span)) => self.divide(buf),
            Some((Token::OpLt, span)) => self.cmplt(buf),
            Some((Token::OpGt, span)) => self.cmpgt(buf),
            Some((Token::OpEq, span)) => self.cmpeq(buf),
            Some((Token::OpNeq, span)) => self.cmpneq(buf),
            Some((Token::LogAnd, span)) => self.logand(buf),
            Some((Token::LogOr, span)) => self.logor(buf),
            Some((Token::LeftParenthesis, span)) | Some((_, span)) => {
                Err(Error::unexpected_token(span))
            }
            None => Err(Error::end_of_token_stream()),
        }
    }

    fn parse_assignment(&mut self, token: SpannedToken) -> ParseResult {
        let start = token.1.start;
        self.lexer.next(); // Eat

        // Extract identifier
        let iden = if let Token::Identifier(i) = token {
            Iden(i)
        } else {
            return Err(Error {
                kind: ErrorKind::InvalidIdentifier,
                span: self.lexer.span(),
            });
        };

        let next = self.lexer.next();
        let mut value = self.parse_expr(next)?;

        loop {
            let peek = self.lexer.peek().clone();
            value = match peek {
                Some(Token::Semicolon) => break,
                None => {
                    return Err(Error {
                        kind: ErrorKind::EndOfTokenStream,
                        span: self.lexer.span(),
                    })
                }
                Some(t) => self.parse_operation(Some(t), value)?,
            };
        }

        self.lexer.next();

        Ok(Stmt::VarAssignment { iden, value }.into())
    }
    // Generate infix
    gen_infix! {
        addition => Add;
        subtract => Subtract;
        multiply => Multiply;
        divide => Divide;
        cmplt => Lt;
        cmpgt => Gt;
        cmpeq => Eq;
        cmpneq => Neq;
        logand => And;
        logor => Or;
    }

    /// Ensure that input token is an expression
    pub(super) fn parse_expr(&mut self, token: Option<SpannedToken>) -> ExprResult {
        let (token, span) = token.ok_or(Error::end_of_token_stream())?;

        match token {
            Token::Boolean(b) => Ok(Expr::new(ExprKind::Literal(Value::Bool(b)), span)),
            Token::Integer(i) => Ok(Expr::new(ExprKind::Literal(Value::Int(i)), span)),
            Token::String(s) => Ok(Expr::new(
                ExprKind::Literal(Value::Str(if self.tdark {
                    s.replace("lang", "script")
                } else {
                    s
                })),
                span,
            )),
            Token::Aboolean(a) => Ok(Expr::new(ExprKind::Literal(Value::Abool(a)), span)),
            Token::Identifier(i) => Ok(Expr::new(
                ExprKind::Identifier(Iden(if self.tdark {
                    i.replace("lang", "script")
                } else {
                    i
                })),
                span,
            )),
            Token::Nul => Ok(Expr::new(ExprKind::Literal(Value::Nul), span)),
            Token::LogNot => {
                let next = self.lexer.next();
                let expr = self.parse_expr(next)?;
                Ok(Expr::new(
                    ExprKind::Not(Box::new(expr)),
                    span.start..expr.span.end,
                ))
            }
            Token::LeftParenthesis => self.parse_paren(),
            _ => Err(Error::unexpected_token(span)),
        }
    }

    /// Parse parenthesieted expression
    pub(super) fn parse_paren(&mut self) -> ExprResult {
        let next = self.lexer.next();
        let mut buf = self.parse_expr(next)?;
        loop {
            let peek = self.lexer.peek().clone();
            buf = match peek {
                Some(Token::RightParenthesis) => {
                    self.lexer.next();
                    return Ok(buf);
                }
                None => return Ok(buf),
                Some(t) => self.parse_operation(Some(t), buf)?,
            };
        }
    }

    /// Parse function call
    fn fn_call(&mut self, token: Token) -> ParseResult {
        let iden = if let Token::Identifier(i) = token {
            Iden(i)
        } else {
            return Err(Error {
                kind: ErrorKind::InvalidIdentifier,
                span: self.lexer.span(),
            });
        };

        self.lexer.next();
        let mut args = Vec::new();
        loop {
            let next = self.lexer.next();

            // No argument function
            if matches!(next, Some(Token::RightParenthesis)) {
                break;
            }

            args.push(self.parse_expr(next)?);
            match self.lexer.next() {
                Some(Token::RightParenthesis) => break,
                Some(Token::Comma) => continue,
                _ => return Err(self.unexpected_token(None)),
            }
        }
        self.require(Token::Semicolon)?;
        Ok(Stmt::FunctionCall { iden, args }.into())
    }
}
