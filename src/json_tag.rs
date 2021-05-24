use std::io::{
    Read,
};
use anyhow::{
    Error,
    Result,
};
use unicode_reader::CodePoints;

pub enum QuoteType {
    None,
    Single,
    Double,
}

pub enum JsonTag {
    LeftCurly,
    RightCurly,
    LeftSquare,
    RightSquare,
    Colon,
    Comma,
    Literal(String, QuoteType),
    Number(String),
}

impl JsonTag {
    pub fn read_json_tag<I>(char_iter: &mut I, buffer: &mut Vec<char>) -> Result<Option<JsonTag>>
        where I: Iterator<Item=Result<char, std::io::Error>>
    {
        //TODO: IMPLEMENT BUFFERED ITERATOR OF CHAR WITH SUPPORT OF PEEK
        let json_tag = loop {
            match char_iter.next() {
                None => break Ok(None),

                Some(Ok(c)) => {
                    match c {
                        c if c.is_whitespace() => continue,
                        '{' => break Ok(Some(JsonTag::LeftCurly)),
                        '}' => break Ok(Some(JsonTag::RightCurly)),
                        '[' => break Ok(Some(JsonTag::LeftSquare)),
                        ']' => break Ok(Some(JsonTag::RightSquare)),
                        ',' => break Ok(Some(JsonTag::Comma)),
                        ':' => break Ok(Some(JsonTag::Colon)),
                        _ => {
                            buffer.push(c);
                            continue;
                        },
                    }
                },

                Some(Err(e)) => break Err(Error::new(e)),
            }
        };

        todo!()
    }

    pub fn parse<R>(reader: R) -> Vec<JsonTag>
        where R: Read
    {
        let mut buffer = Vec::new();
        let mut codepoints = CodePoints::from(reader);
        let json_tag = JsonTag::read_json_tag(&mut codepoints, &mut buffer);

        todo!()
    }
}