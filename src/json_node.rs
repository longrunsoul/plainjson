use anyhow::{
    Result,
    bail,
};

use crate::json_tag::*;

pub struct JsonObjProp {
    pub name: String,
    pub value: JsonNode,
}

pub enum JsonNode {
    PlainNull,
    PlainString(String),
    PlainNumber(String),
    PlainBoolean(bool),
    Array(Vec<JsonNode>),
    Object(Vec<JsonObjProp>),
}

impl JsonNode {
    pub fn parse(json_tags: &[JsonTag]) -> Result<Vec<JsonNode>> {
        let mut i = 0;
        let mut json_nodes = Vec::new();
        while i < json_tags.len() {
            match &json_tags[i] {
                JsonTag::Literal(literal) => {
                    let plain_node = JsonNode::parse_plain(literal);
                    json_nodes.push(plain_node);

                    i += 1;
                    continue;
                }

                JsonTag::LeftCurly => {
                    let right_curly_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftCurly, JsonTag::RightCurly)?;

                    let object_node = JsonNode::parse_object(&json_tags[i..=right_curly_i]);
                    json_nodes.push(object_node);

                    i = right_curly_i + 1;
                    continue;
                }

                JsonTag::LeftSquare => {
                    let right_square_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftSquare, JsonTag::RightSquare)?;

                    let array_node = JsonNode::parse_array(&json_tags[i..=right_square_i])?;
                    json_nodes.push(array_node);

                    i = right_square_i + 1;
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

    fn find_match_tag(json_tags: &[JsonTag], start: usize, left_pair_tag: JsonTag, right_pair_tag: JsonTag) -> Result<usize> {
        let mut i = start;
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
            bail!("matching {} not found for json: {}", JsonTag::to_string(&[right_pair_tag]), JsonTag::to_string(&json_tags[i..]))
        }

        Ok(i)
    }

    fn parse_plain(literal: &str) -> JsonNode {
        match literal {
            str if str.chars().all(|c| c.is_numeric() || c == '.')
                && str.chars().filter(|c| *c == '.').count() <= 1
            => JsonNode::PlainNumber(String::from(str)),

            "true" | "True" | "TRUE" => JsonNode::PlainBoolean(true),
            "false" | "False" | "FALSE" => JsonNode::PlainBoolean(false),

            "null" | "Null" | "NULL" => JsonNode::PlainNull,

            _ => {
                if (
                    (literal.chars().nth(0) == Some('\'') && literal.chars().last() == Some('\''))
                        || (literal.chars().nth(0) == Some('"') && literal.chars().last() == Some('"'))
                ) && literal.len() > 1 {
                    JsonNode::PlainString(String::from(&literal[1..literal.len() - 1]))
                } else {
                    JsonNode::PlainString(String::from(literal))
                }
            }
        }
    }

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
            let mut nodes = match &inner_tags[i] {
                JsonTag::Literal(_) => {
                    i += 1;
                    JsonNode::parse(&inner_tags[i..=i])?
                }
                JsonTag::LeftSquare => {
                    let right_square_i = JsonNode::find_match_tag(inner_tags, i, JsonTag::LeftSquare, JsonTag::RightSquare)?;

                    i = right_square_i + 1;
                    JsonNode::parse(&inner_tags[i..=right_square_i])?
                }
                JsonTag::LeftCurly => {
                    let right_curly_i = JsonNode::find_match_tag(inner_tags, i, JsonTag::LeftCurly, JsonTag::RightCurly)?;

                    i = right_curly_i + 1;
                    JsonNode::parse(&inner_tags[i..=right_curly_i])?
                }

                _ => {
                    i += 1;
                    continue;
                }
            };

            inner_nodes.append(&mut nodes);
        }

        let array_node = JsonNode::Array(inner_nodes);
        Ok(array_node)
    }

    fn parse_object(json_tags: &[JsonTag]) -> JsonNode {
        todo!()
    }
}