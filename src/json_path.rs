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
    fn read_root<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {


        todo!()
    }
    fn read_current<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {


        todo!()
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
                    '$' => JsonPathPart::read_root(peekable_cp),
                    '@' => JsonPathPart::read_current(peekable_cp),
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