// src/parser.rs

use crate::lexer::Token;
use crate::ast::*;  // This imports everything: Statement, Expression, BinaryOperator, LiteralValue, Span, AstError, Program, etc.
use crate::extension::ExtensionRegistry;
use crate::extension::LanguageExtension;
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    source: String,
    extensions: ExtensionRegistry,
}

impl Parser {
    pub fn new(tokens: Vec<Token>, source: String, extensions: ExtensionRegistry) -> Self {
        Self {
            tokens,
            current: 0,
            source,
            extensions,
        }
    }
    
    /// Take the extensions from the parser (for passing to interpreter)
    pub fn take_extensions(&mut self) -> ExtensionRegistry {
        std::mem::take(&mut self.extensions)
    }
    
    pub fn parse(&mut self) -> Result<Program, AstError> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            if let Some(stmt) = self.try_parse_extension()? {
                statements.push(stmt);
            } else {
                statements.push(self.parse_statement()?);
            }
        }
        
        let mut program = Program::new(statements);
        
        for ext in self.extensions.get_extensions() {
            if let Err(e) = ext.post_process_ast(&mut program) {
                return Err(AstError::ParseError(
                    format!("Extension '{}' post-processing failed: {}", ext.name(), e),
                    Span::new(0, 0, 0, 0),
                ));
            }
        }
        
        Ok(program)
    }
    
    fn try_parse_extension(&mut self) -> Result<Option<Statement>, AstError> {
        for ext in self.extensions.get_extensions() {
            if let Some((expr, new_pos)) = ext.parse_extension(&self.tokens, self.current) {
                self.current = new_pos;
                return Ok(Some(Statement::Expression(expr)));
            }
        }
        Ok(None)
    }
    
    fn try_parse_extension_expression(&mut self) -> Result<Option<Expression>, AstError> {
        for ext in self.extensions.get_extensions() {
            if let Some((expr, new_pos)) = ext.parse_extension(&self.tokens, self.current) {
                self.current = new_pos;
                return Ok(Some(expr));
            }
        }
        Ok(None)
    }
    
    // ============ STATEMENT PARSING ============
    
    fn parse_statement(&mut self) -> Result<Statement, AstError> {
        match self.peek() {
            Token::Write => self.parse_write_statement(),
            Token::If => self.parse_if_statement(),
            Token::Loop => self.parse_loop_statement(),
            Token::Func => self.parse_function_definition(),
            Token::Import => self.parse_import_statement(),
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
    
    fn parse_make_statement(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        
        if !matches!(self.peek(), Token::Make) {
            return Err(AstError::ExpectedToken("make".to_string(), span));
        }
        self.advance();
        
        let name = self.consume_identifier();
        
        if !matches!(self.peek(), Token::Equal) {
            return Err(AstError::ExpectedToken("=".to_string(), span));
        }
        self.advance();
        
        let value = self.parse_expression()?;
        
        Ok(Statement::Assignment {
            name,
            value,
            span,
        })
    }
    
    fn parse_write_statement(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume Write
        
        if !matches!(self.peek(), Token::LeftParen) {
            return Err(AstError::ExpectedToken("(".to_string(), span));
        }
        self.advance(); // consume (
        
        let mut args = Vec::new();
        if !matches!(self.peek(), Token::RightParen) {
            args = self.parse_expression_list_paren()?;
        }
        
        if !matches!(self.peek(), Token::RightParen) {
            return Err(AstError::ExpectedToken(")".to_string(), span));
        }
        self.advance(); // consume )
        
        let expr = Expression::FunctionCall {
            name: "write".to_string(),
            arguments: args,
            span,
        };
        
        Ok(Statement::Expression(expr))
    }
    
    fn parse_condition(&mut self) -> Result<Expression, AstError> {
        if !matches!(self.peek(), Token::LessThan) {
            return Err(AstError::ExpectedToken("<".to_string(), Span::new(self.current, self.current, 0, 0)));
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
            return Err(AstError::ExpectedToken(">".to_string(), Span::new(self.current, self.current, 0, 0)));
        }
        
        let mut temp_parser = Parser::new(
            expr_tokens, 
            self.source.clone(),
            std::mem::take(&mut self.extensions),
        );
        let result = temp_parser.parse_expression()
            .map_err(|e| AstError::ParseError(
                format!("Invalid condition: {}", e), 
                Span::new(self.current, self.current, 0, 0)
            ));
        
        self.extensions = temp_parser.take_extensions();
        
        result
    }
    
    fn parse_if_statement(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume If
        
        let condition = self.parse_condition()?;
        let then_branch = self.parse_block()?;
        
        let mut elif_branches = Vec::new();
        while matches!(self.peek(), Token::Elif) {
            self.advance(); // consume Elif
            let elif_condition = self.parse_condition()?;
            let elif_body = self.parse_block()?;
            elif_branches.push((elif_condition, elif_body));
        }
        
        let else_branch = if matches!(self.peek(), Token::Else) {
            self.advance(); // consume Else
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
    
    fn parse_block(&mut self) -> Result<Vec<Statement>, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        
        if !matches!(self.peek(), Token::LeftBrace) {
            return Err(AstError::ExpectedToken("{".to_string(), span));
        }
        self.advance(); // consume {
        
        let mut statements = Vec::new();
        while !self.is_at_end() && !matches!(self.peek(), Token::RightBrace) {
            statements.push(self.parse_statement()?);
        }
        
        if !matches!(self.peek(), Token::RightBrace) {
            return Err(AstError::ExpectedToken("}".to_string(), span));
        }
        self.advance(); // consume }
        
        Ok(statements)
    }
    
    fn parse_loop_statement(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume Loop
        
        let count = match self.peek() {
            Token::Number(n) => {
                let value = *n;
                self.advance();
                value
            }
            _ => return Err(AstError::ExpectedToken("number".to_string(), span)),
        };
        
        let counter_var = if matches!(self.peek(), Token::With) {
            self.advance(); // consume With
            match self.peek() {
                Token::Identifier(name) => {
                    let name = name.clone();
                    self.advance();
                    Some(name)
                }
                _ => return Err(AstError::ExpectedToken("identifier".to_string(), span)),
            }
        } else {
            None
        };
        
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
    
    fn parse_import_statement(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume Import
        
        if !matches!(self.peek(), Token::From) {
            return Err(AstError::ExpectedToken("from".to_string(), span));
        }
        self.advance(); // consume From
        
        let package_name = match self.peek() {
            Token::String(s) => {
                let name = s.clone();
                self.advance();
                name
            }
            _ => return Err(AstError::ExpectedToken("string".to_string(), span)),
        };
        
        if !matches!(self.peek(), Token::LeftBrace) {
            return Err(AstError::ExpectedToken("{".to_string(), span));
        }
        self.advance(); // consume {
        
        let mut items = Vec::new();
        while !matches!(self.peek(), Token::RightBrace) && !self.is_at_end() {
            if let Token::Identifier(name) = self.peek() {
                items.push(name.clone());
                self.advance();
                if matches!(self.peek(), Token::Comma) {
                    self.advance();
                }
            } else {
                return Err(AstError::ExpectedToken("identifier".to_string(), span));
            }
        }
        
        if !matches!(self.peek(), Token::RightBrace) {
            return Err(AstError::ExpectedToken("}".to_string(), span));
        }
        self.advance(); // consume }
        
        Ok(Statement::ImportStatement {
            from: package_name,
            items,
            span,
        })
    }
    
    fn parse_assignment_statement(&mut self) -> Result<Statement, AstError> {
        let name = self.consume_identifier();
        let span = Span::new(self.current, self.current, 0, 0);
        
        if !matches!(self.peek(), Token::Equal) {
            return Err(AstError::ExpectedToken("=".to_string(), span));
        }
        self.advance(); // consume '='
        
        let value = self.parse_expression()?;
        
        Ok(Statement::Assignment {
            name,
            value,
            span,
        })
    }
    
    fn parse_function_definition(&mut self) -> Result<Statement, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume Func
        
        let name = self.consume_identifier();
        
        if !matches!(self.peek(), Token::LessThan) {
            return Err(AstError::ExpectedToken("<".to_string(), span));
        }
        self.advance(); // consume <
        
        let mut parameters = Vec::new();
        if !matches!(self.peek(), Token::GreaterThan) {
            parameters = self.parse_identifier_list()?;
        }
        
        if !matches!(self.peek(), Token::GreaterThan) {
            return Err(AstError::ExpectedToken(">".to_string(), span));
        }
        self.advance(); // consume >
        
        let body = self.parse_block()?;
        
        Ok(Statement::FunctionDefinition {
            name,
            parameters,
            body,
            span,
        })
    }
    
    // ============ EXPRESSION PARSING ============
    
    fn parse_expression(&mut self) -> Result<Expression, AstError> {
        if let Some(expr) = self.try_parse_extension_expression()? {
            return Ok(expr);
        }
        self.parse_binary_expression()
    }
    
    fn parse_binary_expression(&mut self) -> Result<Expression, AstError> {
        let mut left = self.parse_primary_expression()?;
        
        while !self.is_at_end() && self.is_binary_operator(&self.peek()) {
            let operator = self.consume_binary_operator()?;
            let right = self.parse_primary_expression()?;
            let span = left.span().merge(&right.span());
            
            left = Expression::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn parse_primary_expression(&mut self) -> Result<Expression, AstError> {
        match self.peek() {
            Token::Number(_) => self.parse_number_literal(),
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
                    Ok(Expression::Variable(name, span))
                }
            }
            Token::LeftParen => {
                self.advance(); // consume (
                let expr = self.parse_expression()?;
                if !matches!(self.peek(), Token::RightParen) {
                    return Err(AstError::ExpectedToken(")".to_string(), Span::new(self.current, self.current, 0, 0)));
                }
                self.advance(); // consume )
                Ok(expr)
            }
            Token::LeftBracket => {
                self.parse_array_literal()
            }
            Token::LeftBrace => {
                self.parse_object_literal()
            }
            Token::LessThan | Token::GreaterThan | Token::Plus | Token::Minus | 
            Token::Multiply | Token::Divide | Token::EqualEqual | Token::NotEqual |
            Token::LessEqual | Token::GreaterEqual => {
                return Err(AstError::UnexpectedToken(
                    format!("Binary operator {:?} in primary expression, expected a value", self.peek()),
                    Span::new(self.current, self.current, 0, 0),
                ));
            }
            token => {
                Err(AstError::UnexpectedToken(
                    format!("{:?}", token),
                    Span::new(self.current, self.current, 0, 0),
                ))
            }
        }
    }
    
    fn parse_function_call_paren(&mut self) -> Result<Expression, AstError> {
        let name = self.consume_identifier();
        let span = Span::new(self.current, self.current, 0, 0);
        
        if !matches!(self.peek(), Token::LeftParen) {
            return Err(AstError::ExpectedToken("(".to_string(), span));
        }
        self.advance(); // consume (
        
        let mut arguments = Vec::new();
        if !matches!(self.peek(), Token::RightParen) {
            arguments = self.parse_expression_list_paren()?;
        }
        
        if !matches!(self.peek(), Token::RightParen) {
            return Err(AstError::ExpectedToken(")".to_string(), span));
        }
        self.advance(); // consume )
        
        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
    }
    
    fn parse_function_call(&mut self) -> Result<Expression, AstError> {
        let name = self.consume_identifier();
        let span = Span::new(self.current, self.current, 0, 0);
        
        if !matches!(self.peek(), Token::LessThan) {
            return Err(AstError::ExpectedToken("<".to_string(), span));
        }
        self.advance(); // consume <
        
        let mut arguments = Vec::new();
        if !matches!(self.peek(), Token::GreaterThan) {
            arguments = self.parse_expression_list_angle()?;
        }
        
        if !matches!(self.peek(), Token::GreaterThan) {
            return Err(AstError::ExpectedToken(">".to_string(), span));
        }
        self.advance(); // consume >
        
        Ok(Expression::FunctionCall {
            name,
            arguments,
            span,
        })
    }
    
    fn parse_expression_list_paren(&mut self) -> Result<Vec<Expression>, AstError> {
        let mut expressions = Vec::new();
        expressions.push(self.parse_expression()?);
        
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume Comma
            if matches!(self.peek(), Token::RightParen) {
                break;
            }
            expressions.push(self.parse_expression()?);
        }
        
        Ok(expressions)
    }
    
    fn parse_expression_list_angle(&mut self) -> Result<Vec<Expression>, AstError> {
        let mut expressions = Vec::new();
        expressions.push(self.parse_expression()?);
        
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume Comma
            if matches!(self.peek(), Token::GreaterThan) {
                break;
            }
            expressions.push(self.parse_expression()?);
        }
        
        Ok(expressions)
    }
    
    fn parse_identifier_list(&mut self) -> Result<Vec<String>, AstError> {
        let mut identifiers = Vec::new();
        identifiers.push(self.consume_identifier());
        
        while matches!(self.peek(), Token::Comma) {
            self.advance(); // consume Comma
            if matches!(self.peek(), Token::GreaterThan) {
                break;
            }
            identifiers.push(self.consume_identifier());
        }
        
        Ok(identifiers)
    }
    
    fn parse_array_literal(&mut self) -> Result<Expression, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume '['
        
        let mut elements = Vec::new();
        if !matches!(self.peek(), Token::RightBracket) {
            elements.push(self.parse_expression()?);
            while matches!(self.peek(), Token::Comma) {
                self.advance(); // consume Comma
                if matches!(self.peek(), Token::RightBracket) {
                    break;
                }
                elements.push(self.parse_expression()?);
            }
        }
        
        if !matches!(self.peek(), Token::RightBracket) {
            return Err(AstError::ExpectedToken("]".to_string(), Span::new(self.current, self.current, 0, 0)));
        }
        self.advance(); // consume ]
        
        Ok(Expression::Array(elements, span))
    }
    
    fn parse_object_literal(&mut self) -> Result<Expression, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        self.advance(); // consume '{'
        
        let mut properties = HashMap::new();
        while !matches!(self.peek(), Token::RightBrace) && !self.is_at_end() {
            let key = match self.peek() {
                Token::Identifier(name) => name.clone(),
                Token::String(s) => s.clone(),
                _ => return Err(AstError::ExpectedToken("property name".to_string(), Span::new(self.current, self.current, 0, 0))),
            };
            self.advance();
            
            if !matches!(self.peek(), Token::Colon) {
                return Err(AstError::ExpectedToken(":".to_string(), Span::new(self.current, self.current, 0, 0)));
            }
            self.advance(); // consume ':'
            
            let value = self.parse_expression()?;
            properties.insert(key, value);
            
            if matches!(self.peek(), Token::Comma) {
                self.advance(); // consume Comma
            }
        }
        
        if !matches!(self.peek(), Token::RightBrace) {
            return Err(AstError::ExpectedToken("}".to_string(), Span::new(self.current, self.current, 0, 0)));
        }
        self.advance(); // consume }
        
        Ok(Expression::Object(properties, span))
    }
    
    // ============ LITERAL PARSERS ============
    
    fn parse_number_literal(&mut self) -> Result<Expression, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        if let Token::Number(value) = self.advance() {
            Ok(Expression::Literal(LiteralValue::Number(value as f64), span))
        } else {
            Err(AstError::ParseError("Expected number".to_string(), span))
        }
    }
    
    fn parse_string_literal(&mut self) -> Result<Expression, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        if let Token::String(value) = self.advance() {
            Ok(Expression::Literal(LiteralValue::String(value.clone()), span))
        } else {
            Err(AstError::ParseError("Expected string".to_string(), span))
        }
    }
    
    fn parse_boolean_literal(&mut self) -> Result<Expression, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        match self.advance() {
            Token::True => Ok(Expression::Literal(LiteralValue::Boolean(true), span)),
            Token::False => Ok(Expression::Literal(LiteralValue::Boolean(false), span)),
            _ => Err(AstError::ParseError("Expected boolean".to_string(), span)),
        }
    }
    
    // ============ HELPER FUNCTIONS ============
    
    fn consume_identifier(&mut self) -> String {
        if let Token::Identifier(name) = self.advance() {
            name.clone()
        } else {
            String::new()
        }
    }
    
    fn consume_binary_operator(&mut self) -> Result<BinaryOperator, AstError> {
        let span = Span::new(self.current, self.current, 0, 0);
        let operator = match self.advance() {
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
            _ => return Err(AstError::UnexpectedToken(
                format!("Expected binary operator, got {:?}", self.peek()),
                span,
            )),
        };
        
        Ok(operator)
    }
    
    fn is_binary_operator(&self, token: &Token) -> bool {
        matches!(token, 
            Token::Plus | Token::Minus | Token::Multiply | Token::Divide |
            Token::EqualEqual | Token::NotEqual |
            Token::LessThan | Token::LessEqual | Token::GreaterThan | Token::GreaterEqual
        )
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
}