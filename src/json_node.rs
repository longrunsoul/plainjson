//! JSON data type such as null, bool, number, string, array, object.

use std::{
    fmt,
    str::FromStr,
    io::Read,
};
use anyhow::{
    Result,
    bail,
};

use crate::json_tag::*;
use std::fmt::Formatter;

/// JSON object property
#[derive(Debug, PartialEq, Clone)]
pub struct JsonObjProp {
    pub name: String,
    pub value: JsonNode,
}

impl JsonObjProp {
    /// Create JSON object property from name and value.
    pub fn new(name: String, value: JsonNode) -> Self {
        JsonObjProp {
            name,
            value,
        }
    }
}

/// JSON data type
#[derive(Debug, PartialEq, Clone)]
pub enum JsonNode {
    PlainNull,
    PlainString(String),
    PlainNumber(f64),
    PlainBoolean(bool),
    Array(Vec<JsonNode>),
    Object(Vec<JsonObjProp>),
}

impl JsonNode {
    /// Parse a single JSON node from a instance that implements Reader trait.
    pub fn parse_single_node<R>(reader: R) -> Result<JsonNode>
        where R: Read {
        let mut nodes = JsonNode::parse(reader)?;
        if 1 != nodes.len() {
            bail!("more than 1 node found");
        }

        let n = nodes.remove(0);
        Ok(n)
    }

    /// Parse JSON nodes from a instance that implements Reader trait.
    pub fn parse<R>(reader: R) -> Result<Vec<JsonNode>>
        where R: Read {
        let tags = JsonTag::parse(reader)?;
        let nodes = JsonNode::parse_tags(&tags)?;
        Ok(nodes)
    }

    /// Parse JSON nodes from a JSON tag slice.
    pub fn parse_tags(json_tags: &[JsonTag]) -> Result<Vec<JsonNode>> {
        let mut i = 0;
        let mut json_nodes = Vec::new();
        while i < json_tags.len() {
            match &json_tags[i] {
                JsonTag::Literal(literal) => {
                    let plain_node = JsonNode::parse_plain(literal)?;
                    json_nodes.push(plain_node);

                    i += 1;
                    continue;
                }

                JsonTag::LeftSquare => {
                    let right_square_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftSquare, JsonTag::RightSquare)?;

                    let array_node = JsonNode::parse_array(&json_tags[i..=right_square_i])?;
                    json_nodes.push(array_node);

                    i = right_square_i + 1;
                    continue;
                }

                JsonTag::LeftCurly => {
                    let right_curly_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftCurly, JsonTag::RightCurly)?;

                    let object_node = JsonNode::parse_object(&json_tags[i..=right_curly_i])?;
                    json_nodes.push(object_node);

                    i = right_curly_i + 1;
                    continue;
                }

                _ => {
                    i += 1;
                    continue;
                }
            }
        }

        Ok(json_nodes)
    }

    /// Parse a single JSON node from a JSON tag slice, starts at specified index.
    fn parse_next(json_tags: &[JsonTag], start: &mut usize) -> Result<Option<JsonNode>> {
        let i = *start;
        let node = match &json_tags[i] {
            JsonTag::Literal(_) => {
                *start += 1;
                JsonNode::parse_tags(&json_tags[i..=i])?.into_iter().next()
            }
            JsonTag::LeftSquare => {
                let right_square_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftSquare, JsonTag::RightSquare)?;

                *start = right_square_i + 1;
                JsonNode::parse_tags(&json_tags[i..=right_square_i])?.into_iter().next()
            }
            JsonTag::LeftCurly => {
                let right_curly_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftCurly, JsonTag::RightCurly)?;

                *start = right_curly_i + 1;
                JsonNode::parse_tags(&json_tags[i..=right_curly_i])?.into_iter().next()
            }

            _ => {
                *start += 1;
                None
            }
        };

        Ok(node)
    }

    /// Find the index of matching tag from a JSON tag slice, start at specified index.
    fn find_match_tag(json_tags: &[JsonTag], start: usize, left_pair_tag: JsonTag, right_pair_tag: JsonTag) -> Result<usize> {
        let mut i = start + 1;
        let mut mismatch = 0;
        while i < json_tags.len()
            && !(json_tags[i] == right_pair_tag && 0 == mismatch) {
            if json_tags[i] == left_pair_tag {
                mismatch += 1;
            } else if json_tags[i] == right_pair_tag {
                mismatch -= 1;
            }

            i += 1;
        }
        if i >= json_tags.len() {
            bail!("matching {} not found for json: {}", JsonTag::to_string(&[right_pair_tag]), JsonTag::to_string(&json_tags[start..]))
        }

        Ok(i)
    }

    /// Parse a plain data type JSON node(null, bool, number, or string) from a literal string.
    fn parse_plain(literal: &str) -> Result<JsonNode> {
        let plain_node =
            match literal {
                str if str.chars().all(|c| c.is_numeric() || c == '.')
                    && str.chars().filter(|c| *c == '.').count() <= 1
                => JsonNode::PlainNumber(f64::from_str(str)?),

                "true" | "True" | "TRUE" => JsonNode::PlainBoolean(true),
                "false" | "False" | "FALSE" => JsonNode::PlainBoolean(false),

                "null" | "Null" | "NULL" => JsonNode::PlainNull,

                _ => {
                    if (
                        (literal.starts_with('\'') && literal.ends_with('\''))
                            || (literal.starts_with('"') && literal.ends_with('"'))
                    ) && literal.len() > 1 {
                        JsonNode::PlainString(String::from(&literal[1..literal.len() - 1]))
                    } else {
                        JsonNode::PlainString(String::from(literal))
                    }
                }
            };

        Ok(plain_node)
    }

    /// Parse a array data type from a JSON tag slice.
    fn parse_array(json_tags: &[JsonTag]) -> Result<JsonNode> {
        let inner_tags =
            if json_tags.first() == Some(&JsonTag::LeftSquare) && json_tags.last() == Some(&JsonTag::RightSquare) {
                &json_tags[1..json_tags.len() - 1]
            } else {
                json_tags
            };

        let mut i = 0;
        let mut inner_nodes = Vec::new();
        while i < inner_tags.len() {
            let node = JsonNode::parse_next(inner_tags, &mut i)?;
            if node.is_none() {
                continue;
            }

            inner_nodes.push(node.unwrap());

            // skip comma symbol
            if i < inner_tags.len() {
                if let JsonTag::Comma = inner_tags[i] {
                    i += 1;
                }
            }
        }

        let array_node = JsonNode::Array(inner_nodes);
        Ok(array_node)
    }

    /// Parse a object data type from a JSON tag slice.
    fn parse_object(json_tags: &[JsonTag]) -> Result<JsonNode> {
        let inner_tags =
            if json_tags.first() == Some(&JsonTag::LeftCurly) && json_tags.last() == Some(&JsonTag::RightCurly) {
                &json_tags[1..json_tags.len() - 1]
            } else {
                json_tags
            };

        let mut i = 0;
        let mut prop_list = Vec::new();
        while i < inner_tags.len() {
            let prop_name =
                if let JsonTag::Literal(str) = &inner_tags[i] {
                    if (str.starts_with('\'') && str.ends_with('\''))
                        || (str.starts_with('"') && str.ends_with('"')) {
                        &str[1..str.len() - 1]
                    } else {
                        str
                    }
                } else {
                    bail!("object property name must be string: {}", JsonTag::to_string(&inner_tags[i..]))
                };

            // skip colon symbol
            let mut start = i + 1;
            if let JsonTag::Colon = inner_tags[start] {
                start += 1;
            }

            let mut value_node = None;
            while start < inner_tags.len() {
                value_node = JsonNode::parse_next(inner_tags, &mut start)?;
                if value_node.is_none() {
                    continue;
                }

                i = start;
                break;
            };
            if value_node.is_none() {
                bail!("object property value not found: {}", JsonTag::to_string(&inner_tags[i..start]));
            }

            let obj_prop = JsonObjProp::new(String::from(prop_name), value_node.unwrap());
            prop_list.push(obj_prop);

            // skip comma symbol
            if i < inner_tags.len() {
                if let JsonTag::Comma = inner_tags[i] {
                    i += 1;
                }
            }
        }

        Ok(JsonNode::Object(prop_list))
    }

    /// Compose a formatted JSON string representation of a JSON node.
    fn fmt_indent(&self, f: &mut Formatter<'_>, indent_width: usize, no_plain_indent: bool) -> fmt::Result {
        let pretty = f.alternate();
        let ending = if pretty { "\n" } else { "" };
        let comma_separator = if pretty { "," } else { ", " };
        let indent_width = if pretty { indent_width } else { 0 };
        let next_indent_width = if pretty { indent_width + 4 } else { 0 };
        let plain_indent_width = if no_plain_indent { 0 } else { indent_width };
        match self {
            JsonNode::PlainNull => write!(f, "{:indent$}null", "", indent = plain_indent_width)?,
            JsonNode::PlainBoolean(b) => write!(f, "{:indent$}{}", "", b, indent = plain_indent_width)?,
            JsonNode::PlainNumber(n) => write!(f, "{:indent$}{}", "", n, indent = plain_indent_width)?,
            JsonNode::PlainString(s) => write!(f, r#"{:indent$}"{}""#, "", s, indent = plain_indent_width)?,
            JsonNode::Object(prop_list) => {
                write!(f, "{{{}", ending)?;

                for i in 0..prop_list.len() {
                    let prop = &prop_list[i];

                    write!(f, r#"{:indent$}"{}""#, "", prop.name, indent = next_indent_width)?;
                    f.write_str(": ")?;

                    prop.value.fmt_indent(f, next_indent_width, true)?;

                    if i != prop_list.len() - 1 {
                        write!(f, "{}{}", comma_separator, ending)?;
                    }
                }

                write!(f, "{}{:indent$}}}", ending, "", indent = indent_width)?;
            }
            JsonNode::Array(arr) => {
                write!(f, "[{}", ending)?;

                for i in 0..arr.len() {
                    let elem = &arr[i];
                    elem.fmt_indent(f, next_indent_width, false)?;

                    if i != arr.len() - 1 {
                        write!(f, "{}{}", comma_separator, ending)?;
                    }
                }

                write!(f, "{}{:indent$}]", ending, "", indent = indent_width)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for JsonNode {
    /// Implement Display trait for JsonNode
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt_indent(f, 0, false)
    }
}

#[cfg(test)]
mod json_node_tests {
    use std::fmt::Write;
    use anyhow::Result;
    use super::JsonNode;
    use super::JsonObjProp;

    /// Test JSON node parsing using a single-line JSON string.
    #[test]
    fn test_one_line() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let json_node = JsonNode::parse_single_node(json.as_bytes())?;
        assert_eq!(
            json_node,
            JsonNode::Object(
                vec![
                    JsonObjProp::new(String::from(r#"simple"#), JsonNode::PlainNumber(123f64)),
                    JsonObjProp::new(
                        String::from(r#"array"#),
                        JsonNode::Array(
                            vec![
                                JsonNode::PlainString(String::from(r#"a"#)),
                                JsonNode::PlainString(String::from(r#"b"#)),
                                JsonNode::PlainString(String::from(r#"c\""#)),
                            ]
                        ),
                    ),
                    JsonObjProp::new(
                        String::from(r#"object"#),
                        JsonNode::Object(
                            vec![
                                JsonObjProp::new(String::from(r#"prop"#), JsonNode::PlainString(String::from(r#"{true]"#))),
                            ]
                        ),
                    )
                ]
            )
        );

        Ok(())
    }

    /// Test JsonNode Display trait implementation.
    #[test]
    fn test_node_to_string() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "test": [333]}}"#;
        let json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let mut to_str = String::new();
        write!(to_str, "{}", json_node)?;
        assert_eq!(json, &to_str);

        Ok(())
    }

    /// Test JsonNode Display trait implementation with alternate formatter option.
    #[test]
    fn test_node_to_string_alternate() -> Result<()> {
        let json =
            r#"{
    "simple": 123,
    "array": [
        "a",
        "b",
        "c\""
    ],
    "object": {
        "prop": "{true]",
        "test": [
            333
        ]
    }
}"#;
        let json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let mut to_str = String::new();
        write!(to_str, "{:#}", json_node)?;
        assert_eq!(json, &to_str);

        Ok(())
    }
}