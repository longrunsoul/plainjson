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
    pub fn parse(json_tags: &[JsonTag]) -> Vec<JsonNode> {
        todo!()
    }

    fn parse_plain(json_tag: JsonTag) -> JsonNode {
        todo!()
    }

    fn parse_array(json_tags: &[JsonTag]) -> JsonNode {
        todo!()
    }

    fn parse_object(json_tags: &[JsonTag]) -> JsonNode {
        todo!()
    }
}