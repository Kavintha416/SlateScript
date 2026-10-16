// slate-core/src/parser.rs

use crate::lexer::Token;
use crate::ast::*;
use crate::extension::ExtensionRegistry;
use crate::debug::*;
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    source: String,
    source_lines: Vec<String>,
    extensions: ExtensionRegistry,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, source: String, extensions: ExtensionRegistry) -> Self {
        let source_lines: Vec<String> = source.lines().map(|s| s.to_string()).collect();
        Self {
            tokens,
            current: 0,
            source,
            source_lines,
            extensions,
        }
    }

    fn resolved_peek(&self) -> Token {
        self.resolved_peek_at(0)
    }

    fn resolved_peek_at(&self, offset: usize) -> Token {
        let idx = self.current + offset;
        if idx >= self.tokens.len() {
            return Token::EOF;
        }
        let tok = &self.tokens[idx];
        if let Token::Identifier(word) = tok {
            if let Some(ext_name) = self.extensions.lookup_keyword(word) {
                return Token::Extension {
                    name: ext_name.to_string(),
                    keyword: word.clone(),
                };
            }
        }
        tok.clone()
    }

    #[allow(dead_code)]
    fn advance_resolved(&mut self) -> Token {
        let tok = self.resolved_peek();
        self.current += 1;
        tok
    }

    pub fn take_extensions(&mut self) -> ExtensionRegistry {
        std::mem::take(&mut self.extensions)
    }

    pub fn parse(&mut self) -> Result<Program, SlateError> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            // 1. Extension statements get first refusal.
            match self.extensions.try_parse_statement(&self.tokens, self.current) {
                Ok(Some((stmt, new_pos))) => {
                    self.current = new_pos;
                    statements.push(stmt);
                    continue;
                }
                Ok(None) => {}
                Err(msg) => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &msg,
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line()));
                }
            }

            // 2. Extension expressions are wrapped as Expression statements.
            match self.extensions.try_parse_expression(&self.tokens, self.current) {
                Ok(Some((expr, new_pos))) => {
                    self.current = new_pos;
                    statements.push(Statement::Expression(expr));
                    continue;
                }
                Ok(None) => {}
                Err(msg) => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &msg,
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line()));
                }
            }

            // 3. Fall through to core statement parsing.
            statements.push(self.parse_core_statement()?);
        }

        let mut program = Program::new(statements);

        for ext in self.extensions.get_extensions_mut_post_process() {
            if let Err(e) = ext.post_process_ast(&mut program) {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Extension '{}' post-processing failed: {}", ext.name(), e),
                    1,
                    1,
                ));
            }
        }

        Ok(program)
    }

    fn parse_core_statement(&mut self) -> Result<Statement, SlateError> {
        match self.peek() {
            Token::Annotation { .. } => self.parse_annotated_statement(),
            Token::Import => self.parse_import_statement(),
            Token::Write  => self.parse_write_statement(),
            Token::If     => self.parse_if_statement(),
            Token::Loop   => self.parse_loop_statement(),
            Token::Func   => self.parse_function_definition(),
            Token::Make   => self.parse_make_statement(),
            Token::Identifier(_) => {
                if self.peek_next_is_assignable() {
                    self.parse_assignment_statement()
                } else {
                    Ok(Statement::Expression(self.parse_expression()?))
                }
            }
            _ => Ok(Statement::Expression(self.parse_expression()?)),
        }
    }

    /// Parse `#[name]` followed by a statement.
    ///
    /// Only `make` and assignment statements are accepted after an
    /// annotation. Other statements will be rejected by the annotation's
    /// `evaluate` method at runtime.
    fn parse_annotated_statement(&mut self) -> Result<Statement, SlateError> {
        let span_start = self.current;

        let annotation = match self.peek() {
            Token::Annotation { name } => name.clone(),
            _ => unreachable!("parse_annotated_statement called without Annotation token"),
        };
        self.advance();

        // Parse the following statement. Only make/assignment are accepted.
        let inner = match self.peek() {
            Token::Make => self.parse_make_statement()?,
            Token::Identifier(_) if self.peek_next_is_assignable() => {
                self.parse_assignment_statement()?
            }
            _ => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!(
                        "Annotation '#[{}]' must precede a `make` or assignment statement",
                        annotation
                    ),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion(&format!(
                    "#[{}]\nmake my_value {{ field: value }}",
                    annotation
                )));
            }
        };

        let span = Span::new(span_start, self.current, 0, 0);
        Ok(Statement::Annotated {
            annotation,
            inner: Box::new(inner),
            span,
        })
    }

    fn parse_make_statement(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);

        if !matches!(self.peek(), Token::Make) {
            let token_str = format!("{:?}", self.peek());
            let error = if token_str.contains("var") {
                ErrorBuilder::var_instead_of_make(
                    self.current_line(),
                    self.current_column(),
                )
            } else if token_str.contains("let") {
                ErrorBuilder::unknown_term_with_suggestion(
                    "let",
                    "Use 'make' instead of 'let' for variable declarations:\n\
                    make x = 5",
                    self.current_line(),
                    self.current_column(),
                )
            } else if token_str.contains("const") {
                ErrorBuilder::unknown_term_with_suggestion(
                    "const",
                    "Use 'make' for all variable declarations:\n\
                    make x = 5",
                    self.current_line(),
                    self.current_column(),
                )
            } else {
                ErrorBuilder::unknown_term(
                    &token_str,
                    self.current_line(),
                    self.current_column(),
                )
            }
            .with_source(self.get_current_line());
            return Err(error);
        }
        self.advance();

        let name = self.consume_identifier();
        if name.is_empty() {
            return Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected variable name after 'make'",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("make <variable_name> = <value>"));
        }

        // ---- Form A: `make name { ... }` (no '=') ----
        //
        // The brace block is parsed as an object literal and used as the
        // value. This is the form used with `#[struct]`:
        //
        //     #[struct]
        //     make point {
        //         x: 3,
        //         y: 4,
        //     }
        //
        if matches!(self.peek(), Token::LeftBrace) {
            let value = self.parse_object_literal()?;
            return Ok(Statement::Assignment {
                name,
                value,
                span,
            });
        }

        // ---- Form B: `make name = value` ----
        if !matches!(self.peek(), Token::Equal) {
            return Err(ErrorBuilder::generic(
                ErrorCode::BracketError,
                &format!("Expected '=' after 'make {}'", name),
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("make {} = <value>", name)));
        }
        self.advance();

        let value = self.parse_expression()?;

        Ok(Statement::Assignment {
            name,
            value,
            span,
        })
    }

    fn peek_next_is_assignable(&self) -> bool {
        // True if the current token is an identifier that eventually
        // leads to an `=`. Handles both `name = ` and `name.field = `.
        if self.current >= self.tokens.len() {
            return false;
        }
        // Must start with an identifier.
        if !matches!(self.tokens[self.current], Token::Identifier(_)) {
            return false;
        }
        // Scan forward, allowing `.identifier` pairs, until we hit
        // something that isn't a dot.
        let mut i = self.current + 1;
        while i + 1 < self.tokens.len() && matches!(self.tokens[i], Token::Dot) {
            i += 2; // skip Dot and the identifier
        }
        // Now check if the next token is `=`.
        i < self.tokens.len() && matches!(self.tokens[i], Token::Equal)
    }

    fn parse_write_statement(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        if !matches!(self.peek(), Token::LeftParen) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("write(\"hello\")"));
        }
        self.advance();

        let mut args = Vec::new();
        if !matches!(self.peek(), Token::RightParen) {
            args = self.parse_expression_list_paren()?;
        }

        if !matches!(self.peek(), Token::RightParen) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("write(\"hello\")"));
        }
        self.advance();

        let expr = Expression::FunctionCall {
            name: "write".to_string(),
            arguments: args,
            span,
        };

        Ok(Statement::Expression(expr))
    }

    fn parse_condition(&mut self) -> Result<Expression, SlateError> {
        if !matches!(self.peek(), Token::LessThan) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("if <condition> { ... }"));
        }

        let mut depth = 1;
        let mut expr_tokens = Vec::new();

        self.advance();

        while !self.is_at_end() && depth > 0 {
            match self.peek() {
                Token::LessThan => {
                    depth += 1;
                    expr_tokens.push(self.advance());
                }
                Token::GreaterThan => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance();
                        break;
                    }
                    expr_tokens.push(self.advance());
                }
                _ => {
                    expr_tokens.push(self.advance());
                }
            }
        }

        if depth > 0 {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("Missing closing '>' in condition"));
        }

        let mut temp_parser = Parser::new(
            expr_tokens,
            self.source.clone(),
            self.extensions.clone(),
        );

        temp_parser.parse_expression()
            .map_err(|e| ErrorBuilder::generic(
                ErrorCode::BracketError,
                &format!("Invalid condition: {}", e),
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        let condition = self.parse_condition()?;
        let then_branch = self.parse_block()?;

        let mut elif_branches = Vec::new();
        while matches!(self.peek(), Token::Elif) {
            self.advance();
            let elif_condition = self.parse_condition()?;
            let elif_body = self.parse_block()?;
            elif_branches.push((elif_condition, elif_body));
        }

        let else_branch = if matches!(self.peek(), Token::Else) {
            self.advance();
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_branch,
            elif_branches,
            else_branch,
            span,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, SlateError> {
        let _span = Span::new(self.current, self.current, 0, 0);

        if !matches!(self.peek(), Token::LeftBrace) {
            return Err(ErrorBuilder::missing_open_brace(
                "block",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()));
        }
        self.advance();

        let mut statements = Vec::new();
        while !self.is_at_end() && !matches!(self.peek(), Token::RightBrace) {
            // Extensions get first refusal inside blocks too.
            match self.extensions.try_parse_statement(&self.tokens, self.current) {
                Ok(Some((stmt, new_pos))) => {
                    self.current = new_pos;
                    statements.push(stmt);
                    continue;
                }
                Ok(None) => {}
                Err(msg) => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &msg,
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line()));
                }
            }

            match self.extensions.try_parse_expression(&self.tokens, self.current) {
                Ok(Some((expr, new_pos))) => {
                    self.current = new_pos;
                    statements.push(Statement::Expression(expr));
                    continue;
                }
                Ok(None) => {}
                Err(msg) => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &msg,
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line()));
                }
            }

            statements.push(self.parse_core_statement()?);
        }

        if !matches!(self.peek(), Token::RightBrace) {
            return Err(ErrorBuilder::missing_close_brace(
                "block",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()));
        }
        self.advance();

        Ok(statements)
    }

    fn parse_loop_statement(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);

        if !matches!(self.peek(), Token::Loop) {
            return Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                &format!("Expected 'loop', found '{:?}'", self.peek()),
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("loop <count> { ... }"));
        }
        self.advance();

        let count = match self.peek() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                if value <= 0 {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Loop count must be positive, got {}", value),
                        self.current_line(),
                        self.current_column() - 1,
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("Use a positive number: loop 5 { ... }"));
                }
                value
            }
            Token::Float(n) => {
                let value = *n as i64;
                let float_val = *n;
                self.advance();
                if value <= 0 {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Loop count must be positive, got {}", value),
                        self.current_line(),
                        self.current_column() - 1,
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("Use a positive number: loop 5 { ... }"));
                }
                if float_val != value as f64 {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Loop count must be a whole number, got {}", float_val),
                        self.current_line(),
                        self.current_column() - 1,
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("Use a whole number: loop 5 { ... }"));
                }
                value
            }
            Token::Identifier(name) => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Expected number, found identifier '{}'", name),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion(&format!(
                    "Use a number directly: loop 5 {{ ... }}\n\
                    (Note: variable-based loop counts are not yet supported)"
                )));
            }
            _ => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Expected number after 'loop', found '{:?}'", self.peek()),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion("loop <count> { ... }"));
            }
        };

        let counter_var = if matches!(self.peek(), Token::With) {
            self.advance();
            match self.peek() {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    if name.is_empty() {
                        return Err(ErrorBuilder::generic(
                            ErrorCode::UnknownTerm,
                            "Expected variable name after 'with'",
                            self.current_line(),
                            self.current_column() - 1,
                        )
                        .with_source(self.get_current_line())
                        .with_suggestion("loop 5 with i { ... }"));
                    }
                    let reserved = ["func", "make", "if", "else", "loop", "write", "import", "from"];
                    if reserved.contains(&name.as_str()) {
                        return Err(ErrorBuilder::generic(
                            ErrorCode::UnknownTerm,
                            &format!("'{}' is a reserved keyword", name),
                            self.current_line(),
                            self.current_column() - 1,
                        )
                        .with_source(self.get_current_line())
                        .with_suggestion("Use a different variable name"));
                    }
                    Some(name)
                }
                Token::Number(n) => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Expected variable name, found number {}", n),
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("loop 5 with i { ... }"));
                }
                _ => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Expected variable name after 'with', found '{:?}'", self.peek()),
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("loop 5 with i { ... }"));
                }
            }
        } else {
            Some("i".to_string())
        };

        if !matches!(self.peek(), Token::LeftBrace) {
            return Err(ErrorBuilder::missing_open_brace(
                "loop",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("loop {} {{ ... }}", count)));
        }

        let body = self.parse_block()?;

        let var_name = counter_var.unwrap_or_else(|| "i".to_string());

        let condition = Expression::Binary {
            left: Box::new(Expression::Variable(var_name.clone(), span)),
            operator: BinaryOperator::LessThan,
            right: Box::new(Expression::Literal(LiteralValue::Number(count as f64), span)),
            span,
        };

        let increment = Statement::Assignment {
            name: var_name.clone(),
            value: Expression::Binary {
                left: Box::new(Expression::Variable(var_name.clone(), span)),
                operator: BinaryOperator::Add,
                right: Box::new(Expression::Literal(LiteralValue::Number(1.0), span)),
                span,
            },
            span,
        };

        let mut full_body = body;
        full_body.push(increment);

        Ok(Statement::While {
            condition,
            body: full_body,
            counter_var: Some(var_name),
            span,
        })
    }

    fn parse_import_statement(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        if !matches!(self.peek(), Token::From) {
            return Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected 'from' after 'import'",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("import from \"package\" { ... }"));
        }
        self.advance();

        let package_name = match self.peek() {
            Token::String(s) => {
                let name = s.clone();
                self.advance();
                name
            }
            Token::Identifier(s) => {
                let name = s.clone();
                self.advance();
                name
            }
            _ => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Expected package name, found '{:?}'", self.peek()),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion("import from \"package\" { ... }"));
            }
        };

        if matches!(self.peek(), Token::LeftBrace) {
            self.advance();

            let mut items = Vec::new();

            while !self.is_at_end() && !matches!(self.peek(), Token::RightBrace) {
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                    continue;
                }

                // Any identifier is a valid import item now.
                // Extension keywords like `Window` come through as identifiers.
                if let Token::Identifier(name) = self.peek() {
                    let name = name.clone();
                    self.advance();
                    items.push(name);
                } else {
                    self.advance();
                }
            }

            if !matches!(self.peek(), Token::RightBrace) {
                return Err(ErrorBuilder::generic(
                    ErrorCode::BracketError,
                    "Expected '}' to close import block",
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
            self.advance();

            Ok(Statement::ImportStatement {
                from: package_name,
                items,
                span,
            })
        } else {
            Ok(Statement::ImportStatement {
                from: package_name,
                items: Vec::new(),
                span,
            })
        }
    }

    fn parse_assignment_statement(&mut self) -> Result<Statement, SlateError> {
        let name = self.consume_identifier();
        if name.is_empty() {
            return Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected variable name",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("make <variable_name> = <value>"));
        }

        let span = Span::new(self.current, self.current, 0, 0);

        // If we see `.`, this is a property assignment: `name.field = ...`
        // or `name.field.field2 = ...`
        if matches!(self.peek(), Token::Dot) {
            let mut target = Expression::Variable(name, span);
            while matches!(self.peek(), Token::Dot) {
                self.advance();
                let prop = match self.peek() {
                    Token::Identifier(p) => {
                        let s = p.clone();
                        self.advance();
                        s
                    }
                    _ => {
                        return Err(ErrorBuilder::generic(
                            ErrorCode::UnknownTerm,
                            "Expected field name after '.'",
                            self.current_line(),
                            self.current_column(),
                        )
                        .with_source(self.get_current_line()));
                    }
                };
                target = Expression::PropertyAccess {
                    object: Box::new(target),
                    property: prop,
                    span,
                };
            }

            if !matches!(self.peek(), Token::Equal) {
                return Err(ErrorBuilder::generic(
                    ErrorCode::BracketError,
                    "Expected '=' after assignment target",
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
            self.advance();

            let value = self.parse_expression()?;
            return Ok(Statement::PropertyAssignment { target, value, span });
        }

        // Simple `name = value`
        if !matches!(self.peek(), Token::Equal) {
            return Err(ErrorBuilder::generic(
                ErrorCode::BracketError,
                &format!("Expected '=' after '{}'", name),
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("{} = <value>", name)));
        }
        self.advance();

        let value = self.parse_expression()?;

        Ok(Statement::Assignment { name, value, span })
    }

    fn parse_function_definition(&mut self) -> Result<Statement, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        let name = self.consume_identifier();
        if name.is_empty() {
            return Err(ErrorBuilder::unknown_term(
                "missing function name",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("func add<a,b> { ... }"));
        }

        if !matches!(self.peek(), Token::LessThan) {
            return Err(ErrorBuilder::missing_angle_open(
                &name,
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("func {}<...> {{ ... }}", name)));
        }
        self.advance();

        let mut parameters = Vec::new();
        if !matches!(self.peek(), Token::GreaterThan) {
            parameters = self.parse_identifier_list()?;
        }

        if !matches!(self.peek(), Token::GreaterThan) {
            return Err(ErrorBuilder::missing_angle_close(
                &name,
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("func {}<{}> {{ ... }}", name, parameters.join(", "))));
        }
        self.advance();

        if !matches!(self.peek(), Token::LeftBrace) {
            return Err(ErrorBuilder::missing_open_brace(
                &name,
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!(
                "func {}<{}> {{ ... }}",
                name,
                parameters.join(", ")
            )));
        }

        let body = self.parse_block()?;

        Ok(Statement::FunctionDefinition {
            name,
            parameters,
            body,
            span,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, SlateError> {
        match self.extensions.try_parse_expression(&self.tokens, self.current) {
            Ok(Some((expr, new_pos))) => {
                self.current = new_pos;
                return Ok(expr);
            }
            Ok(None) => {}
            Err(msg) => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &msg,
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
        }
        self.parse_binary_expression()
    }

    fn parse_binary_expression(&mut self) -> Result<Expression, SlateError> {
        self.parse_expression_with_precedence(0)
    }

    fn parse_expression_with_precedence(&mut self, min_precedence: u8) -> Result<Expression, SlateError> {
        let mut left = self.parse_primary_expression()?;

        while let Some((op, precedence)) = self.peek_binary_operator() {
            if precedence < min_precedence {
                break;
            }

            self.advance();
            let right = self.parse_expression_with_precedence(precedence + 1)?;
            let span = left.span().merge(&right.span());

            left = Expression::Binary {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
                span,
            };
        }

        Ok(left)
    }

    fn peek_binary_operator(&mut self) -> Option<(BinaryOperator, u8)> {
        let precedence = match self.peek() {
            Token::Multiply | Token::Divide => 3,
            Token::Plus | Token::Minus => 2,
            Token::EqualEqual | Token::NotEqual |
            Token::LessThan | Token::LessEqual |
            Token::GreaterThan | Token::GreaterEqual => 1,
            _ => return None,
        };

        let op = match self.peek() {
            Token::Plus => BinaryOperator::Add,
            Token::Minus => BinaryOperator::Subtract,
            Token::Multiply => BinaryOperator::Multiply,
            Token::Divide => BinaryOperator::Divide,
            Token::EqualEqual => BinaryOperator::Equal,
            Token::NotEqual => BinaryOperator::NotEqual,
            Token::LessThan => BinaryOperator::LessThan,
            Token::LessEqual => BinaryOperator::LessThanOrEqual,
            Token::GreaterThan => BinaryOperator::GreaterThan,
            Token::GreaterEqual => BinaryOperator::GreaterThanOrEqual,
            _ => return None,
        };

        Some((op, precedence))
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, SlateError> {
        match self.peek() {
            Token::Number(_) => self.parse_number_literal(),
            Token::Float(_) => self.parse_float_literal(),
            Token::String(_) => self.parse_string_literal(),
            Token::True | Token::False => self.parse_boolean_literal(),
            Token::Identifier(_) => {
                if self.peek_next_less_than() {
                    self.parse_function_call()
                } else if self.peek_next_paren() {
                    self.parse_function_call_paren()
                } else {
                    let name = self.consume_identifier();
                    let span = Span::new(self.current, self.current, 0, 0);

                    if matches!(self.peek(), Token::Dot) {
                        return self.parse_property_access(Expression::Variable(name, span));
                    }

                    Ok(Expression::Variable(name, span))
                }
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                if !matches!(self.peek(), Token::RightParen) {
                    return Err(ErrorBuilder::bracket_error(
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion("Expected ')' to close expression"));
                }
                self.advance();
                Ok(expr)
            }
            Token::LeftBracket => self.parse_array_literal(),
            Token::LeftBrace => self.parse_object_literal(),
            Token::LessThan | Token::GreaterThan | Token::Plus | Token::Minus |
            Token::Multiply | Token::Divide | Token::EqualEqual | Token::NotEqual |
            Token::LessEqual | Token::GreaterEqual => {
                return Err(ErrorBuilder::bracket_error(
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion(&format!("Expected a value, found '{:?}'", self.peek())));
            }
            token => {
                return Err(ErrorBuilder::unknown_term(
                    &format!("{:?}", token),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion("Check your syntax"));
            }
        }
    }

    fn parse_function_call(&mut self) -> Result<Expression, SlateError> {
        let name = self.consume_identifier();
        let span = Span::new(self.current, self.current, 0, 0);

        if !matches!(self.peek(), Token::LessThan) {
            return Err(ErrorBuilder::missing_angle_open(
                &name,
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("{}<...>", name)));
        }
        self.advance();

        let mut arguments = Vec::new();

        if !matches!(self.peek(), Token::GreaterThan) {
            arguments.push(self.parse_expression()?);

            while matches!(self.peek(), Token::Comma) {
                self.advance();
                if matches!(self.peek(), Token::GreaterThan) {
                    break;
                }
                arguments.push(self.parse_expression()?);
            }
        }

        if !matches!(self.peek(), Token::GreaterThan) {
            return Err(ErrorBuilder::missing_angle_close(
                &name,
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("{}<...>", name)));
        }
        self.advance();

        Ok(Expression::FunctionCall { name, arguments, span })
    }

    fn parse_property_access(&mut self, object: Expression) -> Result<Expression, SlateError> {
        let span = object.span();
        self.advance();

        let property = match self.peek() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Expected property name, found '{:?}'", self.peek()),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
        };

        if matches!(self.peek(), Token::LeftParen) {
            self.advance();

            let mut arguments = Vec::new();
            if !matches!(self.peek(), Token::RightParen) {
                arguments.push(self.parse_expression()?);
                while matches!(self.peek(), Token::Comma) {
                    self.advance();
                    if matches!(self.peek(), Token::RightParen) {
                        break;
                    }
                    arguments.push(self.parse_expression()?);
                }
            }

            if !matches!(self.peek(), Token::RightParen) {
                return Err(ErrorBuilder::bracket_error(
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line())
                .with_suggestion("Expected ')' to close method call"));
            }
            self.advance();

            Ok(Expression::MethodCall {
                object: Box::new(object),
                method: property,
                arguments,
                span: span.merge(&Span::new(self.current, self.current, 0, 0)),
            })
        } else {
            Ok(Expression::PropertyAccess {
                object: Box::new(object),
                property,
                span: span.merge(&Span::new(self.current, self.current, 0, 0)),
            })
        }
    }

    fn parse_function_call_paren_with_name(&mut self, name: String, span: Span) -> Result<Expression, SlateError> {
        if !matches!(self.peek(), Token::LeftParen) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion(&format!("{}(...)", name)));
        }
        self.advance();

        let mut arguments = Vec::new();

        if !matches!(self.peek(), Token::RightParen) {
            arguments.push(self.parse_expression()?);

            while matches!(self.peek(), Token::Comma) {
                self.advance();
                if matches!(self.peek(), Token::RightParen) {
                    break;
                }
                arguments.push(self.parse_expression()?);
            }
        }

        if !matches!(self.peek(), Token::RightParen) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("Expected ')' to close function call"));
        }
        self.advance();

        Ok(Expression::FunctionCall { name, arguments, span })
    }

    fn parse_function_call_paren(&mut self) -> Result<Expression, SlateError> {
        let name = self.consume_identifier();
        let span = Span::new(self.current, self.current, 0, 0);
        self.parse_function_call_paren_with_name(name, span)
    }

    fn parse_expression_list_paren(&mut self) -> Result<Vec<Expression>, SlateError> {
        let mut expressions = Vec::new();

        if matches!(self.peek(), Token::RightParen) {
            return Ok(expressions);
        }

        expressions.push(self.parse_expression()?);

        while matches!(self.peek(), Token::Comma) {
            self.advance();
            if matches!(self.peek(), Token::RightParen) {
                break;
            }
            expressions.push(self.parse_expression()?);
        }

        Ok(expressions)
    }

    fn parse_identifier_list(&mut self) -> Result<Vec<String>, SlateError> {
        let mut identifiers = Vec::new();
        identifiers.push(self.consume_identifier());

        while matches!(self.peek(), Token::Comma) {
            self.advance();
            if matches!(self.peek(), Token::GreaterThan) {
                break;
            }
            identifiers.push(self.consume_identifier());
        }

        Ok(identifiers)
    }

    fn parse_array_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        let mut elements = Vec::new();
        if !matches!(self.peek(), Token::RightBracket) {
            elements.push(self.parse_expression()?);
            while matches!(self.peek(), Token::Comma) {
                self.advance();
                if matches!(self.peek(), Token::RightBracket) {
                    break;
                }
                elements.push(self.parse_expression()?);
            }
        }

        if !matches!(self.peek(), Token::RightBracket) {
            return Err(ErrorBuilder::bracket_error(
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())
            .with_suggestion("Expected ']' to close array"));
        }
        self.advance();

        Ok(Expression::Array(elements, span))
    }

    fn parse_object_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance();

        let mut properties = HashMap::new();
        while !matches!(self.peek(), Token::RightBrace) && !self.is_at_end() {
            let key = match self.peek() {
                Token::Identifier(name) => name.clone(),
                Token::String(s) => s.clone(),
                _ => {
                    return Err(ErrorBuilder::generic(
                        ErrorCode::UnknownTerm,
                        &format!("Expected property name, found '{:?}'", self.peek()),
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line()));
                }
            };
            self.advance();

            if !matches!(self.peek(), Token::Colon) {
                return Err(ErrorBuilder::generic(
                    ErrorCode::BracketError,
                    &format!("Expected ':' after '{}'", key),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
            self.advance();

            let value = self.parse_expression()?;
            properties.insert(key, value);

            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }

        if !matches!(self.peek(), Token::RightBrace) {
            return Err(ErrorBuilder::missing_close_brace(
                "object",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()));
        }
        self.advance();

        Ok(Expression::Object(properties, span))
    }

    fn parse_number_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        if let Token::Number(value) = self.advance() {
            Ok(Expression::Literal(LiteralValue::Number(value as f64), span))
        } else {
            Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected number",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()))
        }
    }

    fn parse_float_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        if let Token::Float(value) = self.advance() {
            Ok(Expression::Literal(LiteralValue::Number(value), span))
        } else {
            Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected float",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()))
        }
    }

    fn parse_string_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        if let Token::String(value) = self.advance() {
            Ok(Expression::Literal(LiteralValue::String(value.clone()), span))
        } else {
            Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected string",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line()))
        }
    }

    fn parse_boolean_literal(&mut self) -> Result<Expression, SlateError> {
        let span = Span::new(self.current, self.current, 0, 0);
        match self.advance() {
            Token::True => Ok(Expression::Literal(LiteralValue::Boolean(true), span)),
            Token::False => Ok(Expression::Literal(LiteralValue::Boolean(false), span)),
            _ => Err(ErrorBuilder::generic(
                ErrorCode::UnknownTerm,
                "Expected boolean",
                self.current_line(),
                self.current_column(),
            )
            .with_source(self.get_current_line())),
        }
    }

    fn consume_identifier(&mut self) -> String {
        if let Token::Identifier(name) = self.advance() {
            name.clone()
        } else {
            String::new()
        }
    }

    fn peek_next_less_than(&self) -> bool {
        if self.current + 1 < self.tokens.len() {
            matches!(self.tokens[self.current + 1], Token::LessThan)
        } else {
            false
        }
    }

    fn peek_next_paren(&self) -> bool {
        if self.current + 1 < self.tokens.len() {
            matches!(self.tokens[self.current + 1], Token::LeftParen)
        } else {
            false
        }
    }

    fn peek_next_assign(&self) -> bool {
        if self.current + 1 < self.tokens.len() {
            matches!(self.tokens[self.current + 1], Token::Equal)
        } else {
            false
        }
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.current].clone();
        self.current += 1;
        token
    }

    fn peek(&self) -> &Token {
        if self.current < self.tokens.len() {
            &self.tokens[self.current]
        } else {
            &Token::EOF
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len() || matches!(self.peek(), Token::EOF)
    }

    fn current_line(&self) -> usize {
        if self.current < self.tokens.len() {
            let pos = self.current;
            let mut line = 1;
            let mut chars_seen = 0;

            for ch in self.source.chars() {
                if chars_seen >= pos {
                    break;
                }
                if ch == '\n' {
                    line += 1;
                }
                chars_seen += 1;
            }

            line
        } else {
            1
        }
    }

    fn current_column(&self) -> usize {
        if self.current < self.tokens.len() {
            let pos = self.current;
            let mut last_newline = 0;

            for (i, _ch) in self.source.chars().enumerate() {
                if i >= pos {
                    break;
                }
                if self.source.chars().nth(i).unwrap_or('\n') == '\n' {
                    last_newline = i + 1;
                }
            }

            pos - last_newline + 1
        } else {
            1
        }
    }

    fn get_current_line(&self) -> String {
        let line_num = self.current_line();
        if line_num > 0 && line_num <= self.source_lines.len() {
            self.source_lines[line_num - 1].clone()
        } else {
            String::new()
        }
    }
}