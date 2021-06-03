

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

pub struct JsonPath {
    pub parts: Vec<JsonPathPart>,
}