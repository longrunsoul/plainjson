use std::io::{
    Read,
};
use anyhow::{
    Result,
};

use crate::peekable_codepoints::*;

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
                                let mut is_escape = false;
                                loop {
                                    match peekable_cp.peek_char(end)? {
                                        None => break,
                                        Some(c) => {
                                            match c {
                                                '\\' => {
                                                    end += 1;
                                                    is_escape = true;
                                                    continue;
                                                }
                                                '{' | '}' | '[' | ']' | ',' | ':' => {
                                                    if !is_escape {
                                                        break;
                                                    }
                                                }
                                                _ => ()
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
        loop{
            let json_tag = JsonTag::read_json_tag(&mut peekable_cp)?;
            if json_tag.is_none() {
                break;
            }

            json_tag_list.push(json_tag.unwrap());
        }

        Ok(json_tag_list)
    }
}