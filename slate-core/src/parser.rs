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
    
    pub fn take_extensions(&mut self) -> ExtensionRegistry {
        std::mem::take(&mut self.extensions)
    }
    
    pub fn parse(&mut self) -> Result<Program, SlateError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            if let Some(stmt) = self.try_parse_extension()? {
                statements.push(stmt);
                continue;
            }
            
            match self.peek() {
                Token::Import => {
                    statements.push(self.parse_import_statement()?);
                }
                Token::Write => {
                    statements.push(self.parse_write_statement()?);
                }
                Token::If => {
                    statements.push(self.parse_if_statement()?);
                }
                Token::Loop => {
                    statements.push(self.parse_loop_statement()?);
                }
                Token::Func => {
                    statements.push(self.parse_function_definition()?);
                }
                Token::Make => {
                    statements.push(self.parse_make_statement()?);
                }
                Token::Identifier(_) => {
                    if self.peek_next_assign() {
                        statements.push(self.parse_assignment_statement()?);
                    } else {
                        statements.push(Statement::Expression(self.parse_expression()?));
                    }
                }
                Token::EOF => break,
                _ => {
                    let token_str = format!("{:?}", self.peek());
                    return Err(ErrorBuilder::unknown_term(
                        &token_str,
                        self.current_line(),
                        self.current_column(),
                    )
                    .with_source(self.get_current_line())
                    .with_suggestion(&format!(
                        "Unexpected token '{}'. Check your syntax.",
                        token_str
                    )));
                }
            }
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
    
    fn try_parse_extension(&mut self) -> Result<Option<Statement>, SlateError> {
        for ext in self.extensions.get_extensions() {
            if let Some((expr, new_pos)) = ext.parse_extension(&self.tokens, self.current) {
                self.current = new_pos;
                return Ok(Some(Statement::Expression(expr)));
            }
        }
        Ok(None)
    }
    
    fn try_parse_extension_expression(&mut self) -> Result<Option<Expression>, SlateError> {
        for ext in self.extensions.get_extensions() {
            if let Some((expr, new_pos)) = ext.parse_extension(&self.tokens, self.current) {
                self.current = new_pos;
                return Ok(Some(expr));
            }
        }
        Ok(None)
    }
    
    fn parse_statement(&mut self) -> Result<Statement, SlateError> {
        match self.peek() {
            Token::Import => self.parse_import_statement(),
            Token::Write => self.parse_write_statement(),
            Token::If => self.parse_if_statement(),
            Token::Loop => self.parse_loop_statement(),
            Token::Func => self.parse_function_definition(),
            Token::Make => self.parse_make_statement(),
            Token::Identifier(_) => {
                if self.peek_next_assign() {
                    self.parse_assignment_statement()
                } else {
                    Ok(Statement::Expression(self.parse_expression()?))
                }
            }
            _ => Ok(Statement::Expression(self.parse_expression()?)),
        }
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
                Token::LessEqual | Token::GreaterEqual | Token::EqualEqual | Token::NotEqual => {
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
        let span = Span::new(self.current, self.current, 0, 0);
        
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
            statements.push(self.parse_statement()?);
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
                        .with_suggestion(&format!("Use a different variable name")));
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
            Token::Identifier(s) if s.to_lowercase() == "slattery" => {
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
                
                let item = match self.peek() {
                    Token::Identifier(name) => {
                        let name = name.clone();
                        self.advance();
                        name
                    }
                    Token::Window => {
                        self.advance();
                        "Window".to_string()
                    }
                    Token::Column => {
                        self.advance();
                        "Column".to_string()
                    }
                    Token::Row => {
                        self.advance();
                        "Row".to_string()
                    }
                    Token::Text => {
                        self.advance();
                        "Text".to_string()
                    }
                    Token::Button => {
                        self.advance();
                        "Button".to_string()
                    }
                    Token::Input => {
                        self.advance();
                        "Input".to_string()
                    }
                    Token::Identity => {
                        self.advance();
                        "Identity".to_string()
                    }
                    Token::Rewrite => {
                        self.advance();
                        "Rewrite".to_string()
                    }
                    Token::OnTap => {
                        self.advance();
                        "on_tap".to_string()
                    }
                    Token::OnClick => {
                        self.advance();
                        "on_click".to_string()
                    }
                    Token::OnChange => {
                        self.advance();
                        "on_change".to_string()
                    }
                    Token::OnInput => {
                        self.advance();
                        "on_input".to_string()
                    }
                    Token::Render => {
                        self.advance();
                        "render".to_string()
                    }
                    _ => {
                        self.advance();
                        continue;
                    }
                };
                items.push(item);
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
        
        Ok(Statement::Assignment {
            name,
            value,
            span,
        })
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
        if let Some(expr) = self.try_parse_angle_component()? {
            return Ok(expr);
        }
        
        if let Some(expr) = self.try_parse_extension_expression()? {
            return Ok(expr);
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
            Token::LeftBracket => {
                self.parse_array_literal()
            }
            Token::LeftBrace => {
                self.parse_object_literal()
            }
            Token::Window | Token::Column | Token::Row | Token::Text |
            Token::Button | Token::Input => {
                self.parse_ui_component()
            }
            Token::Parent | Token::Child => {
                let token = self.advance();
                let name = format!("{:?}", token);
                let span = Span::new(self.current, self.current, 0, 0);
                Ok(Expression::Variable(name, span))
            }
            Token::OnTap | Token::OnClick | Token::OnChange | Token::OnInput => {
                let token = self.advance();
                let name = format!("{:?}", token);
                let span = Span::new(self.current, self.current, 0, 0);
                Ok(Expression::Variable(name, span))
            }
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
    
    fn parse_ui_component(&mut self) -> Result<Expression, SlateError> {
        let component_type = match self.peek() {
            Token::Window => UiComponentType::Window,
            Token::Column => UiComponentType::Column,
            Token::Row => UiComponentType::Row,
            Token::Text => UiComponentType::Text,
            Token::Button => UiComponentType::Button,
            Token::Input => UiComponentType::Input,
            _ => {
                return Err(ErrorBuilder::generic(
                    ErrorCode::UnknownTerm,
                    &format!("Expected UI component type, found '{:?}'", self.peek()),
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
        };
        
        self.advance();
        let span = Span::new(self.current, self.current, 0, 0);
        
        if matches!(self.peek(), Token::LeftBrace) {
            self.advance();
            
            let mut properties = HashMap::new();
            let mut children = Vec::new();
            let mut events = HashMap::new();
            
            while !self.is_at_end() && !matches!(self.peek(), Token::RightBrace) {
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                    continue;
                }
                
                let prop_name = match self.peek() {
                    Token::Identifier(name) => {
                        let n = name.clone();
                        self.advance();
                        n
                    }
                    Token::Text => {
                        self.advance();
                        "value".to_string()
                    }
                    Token::Button => {
                        self.advance();
                        "label".to_string()
                    }
                    Token::OnTap => {
                        self.advance();
                        "on_tap".to_string()
                    }
                    Token::OnClick => {
                        self.advance();
                        "on_click".to_string()
                    }
                    Token::OnChange => {
                        self.advance();
                        "on_change".to_string()
                    }
                    Token::OnInput => {
                        self.advance();
                        "on_input".to_string()
                    }
                    Token::Identity => {
                        self.advance();
                        "Identity".to_string()
                    }
                    Token::Parent => {
                        self.advance();
                        "Parent".to_string()
                    }
                    Token::Child => {
                        self.advance();
                        "Child".to_string()
                    }
                    Token::Render => {
                        self.advance();
                        "Render".to_string()
                    }
                    _ => {
                        self.advance();
                        continue;
                    }
                };
                
                if matches!(self.peek(), Token::Colon) {
                    self.advance();
                    
                    let value = self.parse_expression()?;
                    
                    if prop_name == "on_tap" || prop_name == "on_click" ||
                       prop_name == "on_change" || prop_name == "on_input" {
                        if let Expression::Variable(var_name, _) = &value {
                            events.insert(prop_name, var_name.clone());
                        }
                    } else if prop_name == "Child" || prop_name == "Parent" {
                        children.push(value);
                    } else if prop_name == "Identity" {
                        if let Expression::Literal(LiteralValue::String(id), _) = &value {
                            properties.insert("Identity".to_string(), Expression::Literal(LiteralValue::String(id.clone()), span));
                        } else if let Expression::Variable(var_name, _) = &value {
                            properties.insert("Identity".to_string(), Expression::Literal(LiteralValue::String(var_name.clone()), span));
                        }
                    } else {
                        properties.insert(prop_name, value);
                    }
                } else if prop_name == "Identity" {
                    if let Token::Identifier(id) = self.peek() {
                        let id = id.clone();
                        self.advance();
                        properties.insert("Identity".to_string(), Expression::Literal(LiteralValue::String(id), span));
                    }
                }
                
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                }
            }
            
            if matches!(self.peek(), Token::RightBrace) {
                self.advance();
            } else {
                return Err(ErrorBuilder::missing_close_brace(
                    "UI component",
                    self.current_line(),
                    self.current_column(),
                )
                .with_source(self.get_current_line()));
            }
            
            Ok(Expression::UiComponent {
                component_type,
                identity: None,
                properties,
                children,
                events,
                span,
            })
        } else {
            Ok(Expression::Variable(component_type.as_str().to_string(), span))
        }
    }
    
    fn try_parse_angle_component(&mut self) -> Result<Option<Expression>, SlateError> {
        if !matches!(self.peek(), Token::LessThan) {
            return Ok(None);
        }
        
        let saved_pos = self.current;
        self.advance();
        
        let component_type = match self.peek() {
            Token::Window => Some(UiComponentType::Window),
            Token::Column => Some(UiComponentType::Column),
            Token::Row => Some(UiComponentType::Row),
            Token::Text => Some(UiComponentType::Text),
            Token::Button => Some(UiComponentType::Button),
            Token::Input => Some(UiComponentType::Input),
            _ => None,
        };
        
        if component_type.is_none() {
            self.current = saved_pos;
            return Ok(None);
        }
        
        let component_type = component_type.unwrap();
        self.advance();
        
        if !matches!(self.peek(), Token::GreaterThan) {
            self.current = saved_pos;
            return Ok(None);
        }
        self.advance();
        
        let span = Span::new(saved_pos, self.current, 0, 0);
        
        let mut properties = HashMap::new();
        let mut children = Vec::new();
        let mut events = HashMap::new();
        
        if matches!(self.peek(), Token::LeftBrace) {
            self.advance();
            
            while !self.is_at_end() && !matches!(self.peek(), Token::RightBrace) {
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                    continue;
                }
                
                let prop_name = match self.peek() {
                    Token::Identifier(name) => {
                        let n = name.clone();
                        self.advance();
                        n
                    }
                    Token::OnTap => {
                        self.advance();
                        "on_tap".to_string()
                    }
                    Token::OnClick => {
                        self.advance();
                        "on_click".to_string()
                    }
                    Token::OnChange => {
                        self.advance();
                        "on_change".to_string()
                    }
                    Token::OnInput => {
                        self.advance();
                        "on_input".to_string()
                    }
                    Token::Identity => {
                        self.advance();
                        "Identity".to_string()
                    }
                    Token::Parent => {
                        self.advance();
                        "Parent".to_string()
                    }
                    Token::Child => {
                        self.advance();
                        "Child".to_string()
                    }
                    Token::Text => {
                        self.advance();
                        "value".to_string()
                    }
                    Token::Button => {
                        self.advance();
                        "label".to_string()
                    }
                    _ => {
                        self.advance();
                        continue;
                    }
                };
                
                if matches!(self.peek(), Token::Colon) {
                    self.advance();
                    
                    let value = self.parse_expression()?;
                    
                    if prop_name == "on_tap" || prop_name == "on_click" ||
                       prop_name == "on_change" || prop_name == "on_input" {
                        if let Expression::Variable(var_name, _) = &value {
                            events.insert(prop_name, var_name.clone());
                        }
                    } else if prop_name == "Child" || prop_name == "Parent" {
                        children.push(value);
                    } else if prop_name == "Identity" {
                        if let Expression::Literal(LiteralValue::String(id), _) = &value {
                            properties.insert("Identity".to_string(), Expression::Literal(LiteralValue::String(id.clone()), span));
                        }
                    } else {
                        properties.insert(prop_name, value);
                    }
                } else if prop_name == "Identity" {
                    if let Token::Identifier(id) = self.peek() {
                        let id = id.clone();
                        self.advance();
                        properties.insert("Identity".to_string(), Expression::Literal(LiteralValue::String(id), span));
                    }
                }
                
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                }
            }
            
            if matches!(self.peek(), Token::RightBrace) {
                self.advance();
            }
        }
        
        Ok(Some(Expression::UiComponent {
            component_type,
            identity: None,
            properties,
            children,
            events,
            span,
        }))
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
        
        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
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
            Token::Window | Token::Column | Token::Row | Token::Text |
            Token::Button | Token::Input | Token::Parent | Token::Child | Token::Rewrite => {
                let token = self.advance();
                format!("{:?}", token)
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
        
        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
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