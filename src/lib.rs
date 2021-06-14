//! This library provides:
//! - JsonTag: A low-level JSON tag parser which reads JSON tags from an instance which implements trait std::io::Read
//! - JsonNode: A JSON parser which supports getting or setting value from/to selected JSON nodes by JSONPath
//!
//! Note: Filter of JSONPath is not implemented yet, so currently filter expression is not supported.

mod peekable_codepoints;
mod json_tag;
mod json_node;
mod json_path;
mod filter_expression;

pub use crate::json_tag::JsonTag;
pub use crate::json_node::JsonNode;