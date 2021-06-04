use std::io::Read;
use anyhow::{
    Result,
    bail,
};

use crate::peekable_codepoints::*;

pub enum ArrayIndexSelector {
    Single(usize),
    Range(Option<i32>, Option<i32>),
    Multiple(Vec<usize>),
}

impl ArrayIndexSelector {
    pub fn parse<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self>
        where R: Read {
        todo!()
    }
}

pub enum FilterExpressionOperand {
    PlainNull,
    PlainString(String),
    PlainNumber(f64),
    PlainBoolean(bool),
    Array(Vec<String>),
    Regex(String),
    Expression(Box<FilterExpression>),
    JsonPath(Box<JsonPath>),
}

pub enum FilterExpressionOperator {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
    MatchRegex,
    Negate,
    LogicAnd,
    LogicOr,
    In,
    NotIn,
    SubSetOf,
    Contains,
    Size,
    Empty,
}

pub struct FilterExpression {
    pub operator: Option<FilterExpressionOperator>,
    pub operand_a: FilterExpressionOperand,
    pub operand_b: Option<FilterExpressionOperand>,
}

pub struct JsonPathPart {
    pub path_name: String,
    pub index_selector: Option<ArrayIndexSelector>,
    pub filter: Option<FilterExpression>,
}

impl JsonPathPart {
    fn new(path_name: &str, index_selector: Option<ArrayIndexSelector>, filter: Option<FilterExpression>) -> Self {
        JsonPathPart {
            path_name: String::from(path_name),
            index_selector,
            filter,
        }
    }

    fn read_root_or_current<R>(peekable_cp: &mut PeekableCodePoints<R>, path_name: &str) -> Result<Option<Self>>
        where R: Read {
        let root_part =
            match peekable_cp.peek_char(1)? {
                None => {
                    peekable_cp.skip(1)?;
                    JsonPathPart::new(path_name, None, None)
                }
                Some(c) => {
                    match c {
                        '.' => {
                            peekable_cp.skip(2)?;
                            JsonPathPart::new(path_name, None, None)
                        }
                        '[' => {
                            match peekable_cp.peek_char(2)? {
                                None => bail!("unexpected end: {}", peekable_cp.peek(2)?),
                                Some(c) => {
                                    match c {
                                        '\'' => {
                                            peekable_cp.skip(1)?;
                                            JsonPathPart::new(path_name, None, None)
                                        }
                                        '0'..='9' | ':' => {
                                            peekable_cp.skip(1)?;
                                            let index_selector = ArrayIndexSelector::parse(peekable_cp)?;
                                            JsonPathPart::new(path_name, Some(index_selector), None)
                                        }
                                        _ => bail!("unrecognized json path: {}...", peekable_cp.peek(3)?)
                                    }
                                }
                            }
                        }
                        _ => bail!("unrecognized json path: {}...", peekable_cp.peek(2)?)
                    }
                }
            };

        Ok(Some(root_part))
    }

    fn read_square_notation<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        todo!()
    }
    fn read_dot_notation<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        todo!()
    }
    pub fn read_part<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        match peekable_cp.peek_char(0)? {
            None => Ok(None),
            Some(c) => {
                match c {
                    '$' => JsonPathPart::read_root_or_current(peekable_cp, "$"),
                    '@' => JsonPathPart::read_root_or_current(peekable_cp, "@"),
                    '[' => JsonPathPart::read_square_notation(peekable_cp),
                    _ => JsonPathPart::read_dot_notation(peekable_cp),
                }
            }
        }
    }
}

pub struct JsonPath {
    pub parts: Vec<JsonPathPart>,
}

impl JsonPath {
    fn new(parts: Vec<JsonPathPart>) -> Self {
        JsonPath {
            parts
        }
    }
    pub fn parse(path_str: &str) -> Result<Self> {
        let mut path_parts = Vec::new();
        let mut peekable_cp = PeekableCodePoints::new(path_str.as_bytes());
        loop {
            let part = JsonPathPart::read_part(&mut peekable_cp)?;
            if part.is_none() {
                break;
            }

            path_parts.push(part.unwrap());
        }

        let json_path = JsonPath::new(path_parts);
        Ok(json_path)
    }
}