use std::io::{
    Read,
};
use anyhow::{
    Result,
};

use crate::peekable_codepoints::*;

#[derive(Debug, Eq, PartialEq)]
pub enum JsonTag {
    LeftCurly,
    RightCurly,
    LeftSquare,
    RightSquare,
    Colon,
    Comma,
    Literal(String),
}

impl JsonTag {
    pub fn read_json_tag<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<JsonTag>>
        where R: Read
    {
        let json_tag =
            loop {
                match peekable_cp.peek_char(0)? {
                    None => break None,
                    Some(c) => {
                        match c {
                            c if c.is_whitespace() => {
                                peekable_cp.skip(1)?;
                                continue;
                            }
                            '{' => break Some(JsonTag::LeftCurly),
                            '}' => break Some(JsonTag::RightCurly),
                            '[' => break Some(JsonTag::LeftSquare),
                            ']' => break Some(JsonTag::RightSquare),
                            ',' => break Some(JsonTag::Comma),
                            ':' => break Some(JsonTag::Colon),
                            _ => {
                                let mut end = 0;
                                let mut quote = None;
                                let mut is_escape = false;
                                let mut quote_as_literal = false;
                                loop {
                                    match peekable_cp.peek_char(end)? {
                                        None => break,
                                        Some(c) => {
                                            match c {
                                                '\\' => {
                                                    is_escape = true;

                                                    end += 1;
                                                    continue;
                                                }

                                                '\'' | '"' if !is_escape && !quote_as_literal => {
                                                    match quote {
                                                        None => quote = Some(c),
                                                        Some(q) if q == c => quote = None,
                                                        _ => (),
                                                    }

                                                    end += 1;
                                                    continue;
                                                }

                                                '\r' | '\n' if !quote.is_none() => {
                                                    quote_as_literal = true;

                                                    quote = None;
                                                    is_escape = false;

                                                    end = 0;
                                                    continue;
                                                }

                                                c if c.is_whitespace() && quote.is_none() => break,

                                                '{' | '}' | '[' | ']' | ',' | ':' if !is_escape && quote.is_none() => break,

                                                _ => (),
                                            }
                                        }
                                    }

                                    is_escape = false;
                                    end += 1;
                                }

                                let literal = peekable_cp.pop(end)?;
                                break Some(JsonTag::Literal(literal));
                            }
                        }
                    }
                }
            };

        match json_tag {
            Some(JsonTag::LeftCurly)
            | Some(JsonTag::RightCurly)
            | Some(JsonTag::LeftSquare)
            | Some(JsonTag::RightSquare)
            | Some(JsonTag::Comma)
            | Some(JsonTag::Colon)
            => peekable_cp.skip(1)?,

            _ => (),
        }

        Ok(json_tag)
    }

    pub fn parse<R>(reader: R) -> Result<Vec<JsonTag>>
        where R: Read
    {
        let mut json_tag_list = Vec::new();
        let mut peekable_cp = PeekableCodePoints::new(reader);
        loop {
            let json_tag = JsonTag::read_json_tag(&mut peekable_cp)?;
            if json_tag.is_none() {
                break;
            }

            json_tag_list.push(json_tag.unwrap());
        }

        Ok(json_tag_list)
    }
}

#[cfg(test)]
mod json_tag_tests {
    use super::*;

    #[test]
    fn test_one_line() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        assert_eq!(
            json_tag_list,
            vec![
                // {
                JsonTag::LeftCurly,

                // "simple": 123
                JsonTag::Literal(String::from(r#""simple""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#"123"#)),

                // ,
                JsonTag::Comma,

                // "array": ["a", "b", "c\""]
                JsonTag::Literal(String::from(r#""array""#)),
                JsonTag::Colon,
                JsonTag::LeftSquare,
                JsonTag::Literal(String::from(r#""a""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""b""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""c\"""#)),
                JsonTag::RightSquare,

                // ,
                JsonTag::Comma,

                // "object": {"prop": "{true]"}
                JsonTag::Literal(String::from(r#""object""#)),
                JsonTag::Colon,
                JsonTag::LeftCurly,
                JsonTag::Literal(String::from(r#""prop""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#""{true]""#)),
                JsonTag::RightCurly,

                // }
                JsonTag::RightCurly,
            ]
        );
        Ok(())
    }

    #[test]
    fn test_multi_line() -> Result<()> {
        let json = r#"
{
    "simple": 123,
    "array": [
        "a",
        "b",
        "c\""
    ],
    "object": {
        "prop": "{true]"
    }
}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        assert_eq!(
            json_tag_list,
            vec![
                // {
                JsonTag::LeftCurly,

                // "simple": 123
                JsonTag::Literal(String::from(r#""simple""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#"123"#)),

                // ,
                JsonTag::Comma,

                // "array": ["a", "b", "c\""]
                JsonTag::Literal(String::from(r#""array""#)),
                JsonTag::Colon,
                JsonTag::LeftSquare,
                JsonTag::Literal(String::from(r#""a""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""b""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""c\"""#)),
                JsonTag::RightSquare,

                // ,
                JsonTag::Comma,

                // "object": {"prop": "{true]"}
                JsonTag::Literal(String::from(r#""object""#)),
                JsonTag::Colon,
                JsonTag::LeftCurly,
                JsonTag::Literal(String::from(r#""prop""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#""{true]""#)),
                JsonTag::RightCurly,

                // }
                JsonTag::RightCurly,
            ]
        );
        Ok(())
    }

    #[test]
    fn test_multi_line_malformed() -> Result<()> {
        let json = r#"
{
    "simple": 123,
    "array": [
        "a",
        "b",
        "c\""
    ],
    "obj
    ect": {
        "prop": "{true]"
    }
}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        assert_eq!(
            json_tag_list,
            vec![
                // {
                JsonTag::LeftCurly,

                // "simple": 123
                JsonTag::Literal(String::from(r#""simple""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#"123"#)),

                // ,
                JsonTag::Comma,

                // "array": ["a", "b", "c\""]
                JsonTag::Literal(String::from(r#""array""#)),
                JsonTag::Colon,
                JsonTag::LeftSquare,
                JsonTag::Literal(String::from(r#""a""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""b""#)), JsonTag::Comma, JsonTag::Literal(String::from(r#""c\"""#)),
                JsonTag::RightSquare,

                // ,
                JsonTag::Comma,

                // "obj
                // ect": {"prop": "{true]"}
                JsonTag::Literal(String::from(r#""obj"#)),
                JsonTag::Literal(String::from(r#"ect""#)),
                JsonTag::Colon,
                JsonTag::LeftCurly,
                JsonTag::Literal(String::from(r#""prop""#)), JsonTag::Colon, JsonTag::Literal(String::from(r#""{true]""#)),
                JsonTag::RightCurly,

                // }
                JsonTag::RightCurly,
            ]
        );
        Ok(())
    }
}