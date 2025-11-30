use super::types::{Filter, Operator, Value};

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Field(String),
    Colon,
    Value(String),
    LParen,
    RParen,
    LBracket,
    RBracket,
    And,
    Or,
    To,
    Not,
    Gte,
    Lte,
    Gt,
    Lt,
}

struct Tokenizer {
    input: Vec<char>,
    pos: usize,
}

impl Tokenizer {
    fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, predicate: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while let Some(ch) = self.peek() {
            if predicate(ch) {
                result.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        result
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while self.peek().is_some() {
            self.skip_whitespace();
            let ch = match self.peek() {
                Some(c) => c,
                None => break,
            };

            match ch {
                '(' => {
                    self.advance();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.advance();
                    tokens.push(Token::RParen);
                }
                '[' => {
                    self.advance();
                    tokens.push(Token::LBracket);
                }
                ']' => {
                    self.advance();
                    tokens.push(Token::RBracket);
                }
                ':' => {
                    self.advance();
                    // Check for operators after colon
                    self.skip_whitespace();
                    if let Some(next) = self.peek() {
                        match next {
                            '>' => {
                                self.advance();
                                if self.peek() == Some('=') {
                                    self.advance();
                                    tokens.push(Token::Gte);
                                } else {
                                    tokens.push(Token::Gt);
                                }
                                continue;
                            }
                            '<' => {
                                self.advance();
                                if self.peek() == Some('=') {
                                    self.advance();
                                    tokens.push(Token::Lte);
                                } else {
                                    tokens.push(Token::Lt);
                                }
                                continue;
                            }
                            _ => {}
                        }
                    }
                    tokens.push(Token::Colon);
                }
                '!' | '-' => {
                    self.advance();
                    tokens.push(Token::Not);
                }
                _ if ch.is_alphanumeric() || ch == '_' => {
                    let word = self.read_while(|c| c.is_alphanumeric() || c == '_' || c == '.');
                    match word.to_uppercase().as_str() {
                        "AND" => tokens.push(Token::And),
                        "OR" => tokens.push(Token::Or),
                        "TO" => tokens.push(Token::To),
                        _ => {
                            // If followed by colon, it's a field name
                            self.skip_whitespace();
                            if self.peek() == Some(':') {
                                tokens.push(Token::Field(word));
                            } else {
                                tokens.push(Token::Value(word));
                            }
                        }
                    }
                }
                _ => {
                    return Err(format!("Unexpected character: {}", ch));
                }
            }
        }

        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        token
    }

    fn parse(&mut self) -> Result<Vec<Filter>, String> {
        let mut filters = Vec::new();

        while self.peek().is_some() {
            let filter = self.parse_filter()?;
            filters.push(filter);

            // Skip AND tokens (implicit between terms)
            if matches!(self.peek(), Some(Token::And)) {
                self.advance();
            }
        }

        Ok(filters)
    }

    fn parse_filter(&mut self) -> Result<Filter, String> {
        // Check for negation
        let negated = if matches!(self.peek(), Some(Token::Not)) {
            self.advance();
            true
        } else {
            false
        };

        // Expect field name
        let field = match self.advance() {
            Some(Token::Field(f)) => f,
            _ => return Err("Expected field name".to_string()),
        };

        // Expect colon or operator
        let operator = match self.peek() {
            Some(Token::Colon) => {
                self.advance();
                Operator::Eq
            }
            Some(Token::Gte) => {
                self.advance();
                Operator::Gte
            }
            Some(Token::Lte) => {
                self.advance();
                Operator::Lte
            }
            Some(Token::Gt) => {
                self.advance();
                Operator::Gt
            }
            Some(Token::Lt) => {
                self.advance();
                Operator::Lt
            }
            _ => return Err("Expected colon or operator".to_string()),
        };

        // Parse values
        let (final_operator, values) = match operator {
            Operator::Eq => self.parse_value_or_group()?,
            _ => {
                // For comparison operators, expect single value
                (operator, vec![self.parse_single_value()?])
            }
        };

        Ok(Filter::new(field, final_operator, values, negated))
    }

    fn parse_value_or_group(&mut self) -> Result<(Operator, Vec<Value>), String> {
        match self.peek() {
            Some(Token::LParen) => Ok((Operator::Eq, self.parse_group()?)),
            Some(Token::LBracket) => {
                let values = self.parse_range()?;
                Ok((Operator::Range, values))
            }
            _ => Ok((Operator::Eq, vec![self.parse_single_value()?])),
        }
    }

    fn parse_group(&mut self) -> Result<Vec<Value>, String> {
        // Consume (
        self.advance();

        let mut values = Vec::new();
        values.push(self.parse_single_value()?);

        while matches!(self.peek(), Some(Token::Or)) {
            self.advance(); // consume OR
            values.push(self.parse_single_value()?);
        }

        // Expect )
        match self.advance() {
            Some(Token::RParen) => Ok(values),
            _ => Err("Expected closing parenthesis".to_string()),
        }
    }

    fn parse_range(&mut self) -> Result<Vec<Value>, String> {
        // Consume [
        self.advance();

        let min = self.parse_single_value()?;

        // Expect TO
        match self.advance() {
            Some(Token::To) => {}
            _ => return Err("Expected TO in range".to_string()),
        }

        let max = self.parse_single_value()?;

        // Expect ]
        match self.advance() {
            Some(Token::RBracket) => {}
            _ => return Err("Expected closing bracket".to_string()),
        }

        // Return as two values with Range operator (handled by caller)
        Ok(vec![min, max])
    }

    fn parse_single_value(&mut self) -> Result<Value, String> {
        match self.advance() {
            Some(Token::Value(v)) => {
                // Try to parse as number
                if let Ok(i) = v.parse::<i64>() {
                    Ok(Value::Integer(i))
                } else if let Ok(f) = v.parse::<f64>() {
                    Ok(Value::Number(f))
                } else if v.eq_ignore_ascii_case("true") {
                    Ok(Value::Boolean(true))
                } else if v.eq_ignore_ascii_case("false") {
                    Ok(Value::Boolean(false))
                } else {
                    Ok(Value::String(v))
                }
            }
            _ => Err("Expected value".to_string()),
        }
    }
}

pub fn parse_dsl(query: &str) -> Result<Vec<Filter>, String> {
    let mut tokenizer = Tokenizer::new(query);
    let tokens = tokenizer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}
