// use crate::json_path::*;
//
// #[derive(Debug, PartialEq)]
// pub enum FilterExpressionOperand {
//     PlainNull,
//     PlainString(String),
//     PlainNumber(f64),
//     PlainBoolean(bool),
//     Array(Vec<String>),
//     Regex(String),
//     Expression(Box<FilterExpression>),
//     JsonPath(Box<JsonPath>),
// }
//
// #[derive(Debug, PartialEq)]
// pub enum FilterExpressionOperator {
//     Equal,
//     NotEqual,
//     GreaterThan,
//     GreaterThanOrEqual,
//     LessThan,
//     LessThanOrEqual,
//     MatchRegex,
//     Negate,
//     LogicAnd,
//     LogicOr,
//     In,
//     NotIn,
//     SubSetOf,
//     Contains,
//     Size,
//     Empty,
// }
//
// #[derive(Debug, PartialEq)]
// pub struct FilterExpression {
//     pub operator: Option<FilterExpressionOperator>,
//     pub operand_a: FilterExpressionOperand,
//     pub operand_b: Option<FilterExpressionOperand>,
// }