use anyhow::{
    Result,
    bail,
};

use crate::json_tag::*;

#[derive(Debug, Eq, PartialEq)]
pub struct JsonObjProp {
    pub name: String,
    pub value: JsonNode,
}

impl JsonObjProp {
    pub fn new(name: String, value: JsonNode) -> Self {
        JsonObjProp {
            name,
            value,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
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

    fn parse_next(json_tags: &[JsonTag], start: &mut usize) -> Result<Option<JsonNode>> {
        let i = *start;
        let node = match &json_tags[i] {
            JsonTag::Literal(_) => {
                *start += 1;
                JsonNode::parse(&json_tags[i..=i])?.into_iter().next()
            }
            JsonTag::LeftSquare => {
                let right_square_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftSquare, JsonTag::RightSquare)?;

                *start = right_square_i + 1;
                JsonNode::parse(&json_tags[i..=right_square_i])?.into_iter().next()
            }
            JsonTag::LeftCurly => {
                let right_curly_i = JsonNode::find_match_tag(json_tags, i, JsonTag::LeftCurly, JsonTag::RightCurly)?;

                *start = right_curly_i + 1;
                JsonNode::parse(&json_tags[i..=right_curly_i])?.into_iter().next()
            }

            _ => {
                *start += 1;
                None
            }
        };

        Ok(node)
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
                    if (str.chars().nth(0) == Some('\'') && str.chars().last() == Some('\''))
                        || (str.chars().nth(0) == Some('"') && str.chars().last() == Some('"')) {
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
}

#[cfg(test)]
mod json_node_tests {
    use super::*;

    #[test]
    fn test_one_line() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let json_node_list = JsonNode::parse(&json_tag_list)?;
        assert_eq!(
            json_node_list,
            vec![
                JsonNode::Object(
                    vec![
                        JsonObjProp::new(String::from(r#"simple"#), JsonNode::PlainNumber(String::from(r#"123"#))),
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
            ]
        );
        
        Ok(())
    }
}