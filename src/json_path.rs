use std::{
    io::Read,
    str::FromStr,
};
use anyhow::{
    Result,
    bail,
};

use crate::peekable_codepoints::*;

#[derive(Debug, Eq, PartialEq)]
pub enum PartFragType {
    None,
    RootPathName,
    CurrentPathName,
    DotNotationPathName,
    BracketNotationPathName,
    ElementSelector,
    Filter,
}

impl PartFragType {
    pub fn identify_frag<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self> {
        let frag_type =
            match peekable_cp.peek_char(0)? {
                None => PartFragType::None,
                Some(c) => {
                    match c {
                        '$' => PartFragType::RootPathName,
                        '@' => PartFragType::CurrentPathName,
                        '[' => {
                            match peekable_cp.peek_char(1)? {
                                None => bail!("unexpected end: {}", peekable_cp.peek(1)?),
                                Some(c) => {
                                    match c {
                                        '\'' => PartFragType::BracketNotationPathName,
                                        '0'..='9' | '-' | ':' => PartFragType::ElementSelector,
                                        '?' => PartFragType::Filter,
                                        _ => bail!("unrecognized json path part fragment: {}...", peekable_cp.peek(2)?)
                                    }
                                }
                            }
                        }
                        _ => PartFragType::DotNotationPathName,
                    }
                }
            };

        Ok(frag_type)
    }
}

pub enum ArrayElementSelector {
    Single(usize),
    Range(Option<i32>, Option<i32>),
    Multiple(Vec<usize>),
}

impl ArrayElementSelector {
    fn trim_brackets(elem_selector_str: String) -> String {
        let inner_str =
            if elem_selector_str.starts_with("[") && elem_selector_str.ends_with("]") {
                elem_selector_str.chars().skip(1).take(elem_selector_str.chars().count() - 2).collect()
            } else {
                elem_selector_str
            };
        inner_str
    }

    fn parse_single(elem_selector_str: String) -> Result<Self> {
        let index_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        let index = usize::from_str(&index_str)?;
        Ok(ArrayElementSelector::Single(index))
    }

    fn parse_range(elem_selector_str: String) -> Result<Self> {
        let range_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        let colon_index = range_str.chars().position(|c| c == ':').unwrap();
        let range_left_str: String = range_str.chars().take(colon_index).collect();
        let range_right_str: String = range_str.chars().skip(colon_index + 1).collect();

        let range_left =
            if range_left_str.is_empty() {
                None
            } else {
                Some(i32::from_str(&range_left_str)?)
            };
        let range_right =
            if range_right_str.is_empty() {
                None
            } else {
                Some(i32::from_str(&range_right_str)?)
            };
        Ok(ArrayElementSelector::Range(range_left, range_right))
    }

    fn parse_multiple(elem_selector_str: String) -> Result<Self> {
        let mut indexes = Vec::new();

        let mut number_str = String::new();
        let num_list_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        for c in num_list_str.chars() {
            if c == ',' {
                let i = usize::from_str(&number_str)?;
                indexes.push(i);

                number_str = String::new();
            } else {
                number_str.push(c);
            }
        }

        Ok(ArrayElementSelector::Multiple(indexes))
    }

    pub fn parse<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self> {
        let mut i = 0;
        let mut has_comma = false;
        let mut has_colon = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)),
                Some(c) => {
                    match c {
                        ']' => break,
                        ',' => has_comma = true,
                        ':' => has_colon = true,
                        '0'..='9' | '-' => (),
                        _ => bail!("unrecognized array element selector: {}...", peekable_cp.peek(i+1)?),
                    }
                }
            }

            i += 1;
        }

        let elem_selector_str = peekable_cp.pop(i + 1)?;
        let elem_selector =
            if has_comma {
                ArrayElementSelector::parse_multiple(elem_selector_str)?
            } else if has_colon {
                ArrayElementSelector::parse_range(elem_selector_str)?
            } else {
                ArrayElementSelector::parse_single(elem_selector_str)?
            };

        Ok(elem_selector)
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
    pub index_selector: Option<ArrayElementSelector>,
    pub filter: Option<FilterExpression>,
}

impl JsonPathPart {
    fn new(path_name: &str, index_selector: Option<ArrayElementSelector>, filter: Option<FilterExpression>) -> Self {
        JsonPathPart {
            path_name: String::from(path_name),
            index_selector,
            filter,
        }
    }

    pub fn parse_dot_notation_path_name<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<String> {
        let mut i = 0;
        let mut is_escape = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => break,
                Some(c) => {
                    match c {
                        '\\' => {
                            is_escape = true;

                            i += 1;
                            continue;
                        }
                        '.' | '[' if !is_escape => break,
                        _ => (),
                    }
                }
            }

            is_escape = false;
            i += 1;
        }
        if 0 == i {
            bail!("empty json path part fragment");
        }

        let path_name = peekable_cp.pop(i)?;
        Ok(path_name)
    }

    pub fn parse_bracket_notation_path_name<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<String> {
        let mut i = 0;
        let mut is_escape = false;
        let mut in_quote = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)?),
                Some(c) => {
                    match c {
                        '\\' => {
                            is_escape = true;

                            i += 1;
                            continue;
                        }
                        '\'' if !is_escape => {
                            in_quote = !in_quote;
                            if false == in_quote {
                                match peekable_cp.peek_char(i + 1) {
                                    None => bail!("unexpected end: {}", peekable_cp.peek(i + 1)?),
                                    Some(c) => {
                                        match c {
                                            ']' => break,
                                            _ => bail!("expecting ] at the end: {}", peekable_cp.peek(i + 2)?),
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }

            i += 1;
            is_escape = false;
        }

        let mut path_name_w_bracket = peekable_cp.pop(i + 1)?;
        if path_name_w_bracket.starts_with("['") && path_name_w_bracket.ends_with("']") {
            path_name_w_bracket = String::from(&path_name_w_bracket[2..i - 1]);
        }

        Ok(path_name_w_bracket)
    }

    pub fn parse_next<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        let path_name;
        let mut elem_selector = None;
        let mut filter = None;

        // TODO: IMPLEMENT DETAILED CODE
        let frag_type = PartFragType::identify_frag(peekable_cp)?;
        match frag_type {
            None => return Ok(None),
            PartFragType::RootPathName => path_name = String::from("$"),
            PartFragType::CurrentPathName => path_name = String::from("@"),
            PartFragType::DotNotationPathName => path_name = JsonPathPart::parse_dot_notation_path_name(peekable_cp)?,
            PartFragType::BracketNotationPathName => path_name = JsonPathPart::parse_bracket_notation_path_name(peekable_cp)?,
            _ => bail!("unexpected json path part type: {}", frag_type),
        }

        let next_frag_type = PartFragType::identify_frag(peekable_cp)?;
        match next_frag_type {
            PartFragType::ElementSelector => elem_selector = Some(ArrayElementSelector::parse(peekable_cp)?),
            PartFragType::Filter => {}
            _ => (),
        }

        let last_frag_type = PartFragType::identify_frag(peekable_cp)?;
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

            if Some('.') == peekable_cp.peek_char(0)? {
                peekable_cp.skip(1);
            }
        }

        let json_path = JsonPath::new(path_parts);
        Ok(json_path)
    }
}