use std::io::Read;
use anyhow::{
    Result,
    bail,
};

use crate::peekable_codepoints::*;

pub enum PartFragType {
    None,
    RootPathName,
    CurrentPathName,
    DotNotationPathName,
    SquareNotationPathName,
    ElementSelector,
    Filter,
}

impl PartFragType {
    pub fn identiry_frag<R>(peekable_cp: &PeekableCodePoints<R>, start: usize) -> Self {
        todo!()
    }
}

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
    fn new(path_name: &str, index_selector: Option<ArrayIndexSelector>, filter: Option<FilterExpression>) -> Self {
        JsonPathPart {
            path_name: String::from(path_name),
            index_selector,
            filter,
        }
    }
    pub fn parse_next<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        let mut path_name = String::default();
        let mut elem_selector = None;
        let mut filter = None;

        // TODO: IMPLEMENT DETAILED CODE
        let frag_type = PartFragType::identiry_frag(&peekable_cp);
        match frag_type {
            None => return Ok(None),
            PartFragType::RootPathName | PartFragType::CurrentPathName => {}
            PartFragType::DotNotationPathName => {}
            PartFragType::SquareNotationPathName => {}
            _ => bail!(""),
        }

        let next_frag_type = PartFragType::identiry_frag(&peekable_cp);
        match next_frag_type {
            PartFragType::ElementSelector => {}
            PartFragType::Filter => {}
            _ => (),
        }

        let last_frag_type = PartFragType::identiry_frag(&peekable_cp);
        match last_frag_type {
            PartFragType::Filter => {}
            _ => (),
        }

        let part = JsonPathPart::new(&path_name, elem_selector, filter);
        Ok(Some(part))
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
            let part = JsonPathPart::parse_next(&mut peekable_cp)?;
            if part.is_none() {
                break;
            }

            path_parts.push(part.unwrap());
        }

        let json_path = JsonPath::new(path_parts);
        Ok(json_path)
    }
}