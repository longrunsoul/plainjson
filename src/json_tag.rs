
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