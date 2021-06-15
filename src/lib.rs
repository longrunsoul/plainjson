//! This library simply provides:
//! - JsonTag: A low-level JSON tag parser which reads JSON tags from an instance which implements trait std::io::Read
//! - JsonNode: A JSON parser which supports getting or setting value from/to selected JSON nodes by JSONPath
//!
//! Note: Filter expression in JSONPath is not implemented yet, so currently filter expression is not supported.
//!
//! Getting value by JSONPath is like:
//! ```
//! use plainjson::JsonNode;
//!
//! fn get_value_by_json_path() {
//!     let json = r#"{"a": 123, "b": {"c": "hello"}}"#;
//!     let mut json = JsonNode::parse_single_node(json.as_bytes()).unwrap();
//!     let c = json.get_str("$.b.c").unwrap();
//!     assert_eq!(c, Some(String::from("hello")));
//! }
//! ```
//!
//! Setting value by JSONPath is like:
//! ```
//! use plainjson::JsonNode;
//!
//! fn set_value_by_json_path() {
//!     let json = r#"{"a": 123, "b": [3, 2, 1]}"#;
//!     let mut json = JsonNode::parse_single_node(json.as_bytes()).unwrap();
//!     json.set_bool("$.b[1]", true).unwrap();
//!
//!     assert_eq!(json.to_string(), r#"{"a": 123, "b": [3, true, 1]}"#)
//! }
//! ```
//!
//! If you need to access low-level JSON tags, use JsonTag:
//! ```
//! use plainjson::JsonTag;
//!
//! fn fix_json() {
//!     let json = r#"{"a": test, "b": "world"}"#;
//!     let mut tags = JsonTag::parse(json.as_bytes()).unwrap();
//!     tags[3] = JsonTag::Literal(String::from(r#""test""#));
//!
//!     assert_eq!(JsonTag::to_string(&tags), r#"{"a": "test", "b": "world"}"#);
//! }
//! ```
//!
//! This library is licensed under <a href="LICENSE">MIT license</a>.

mod peekable_codepoints;
mod json_tag;
mod json_node;
mod json_path;
mod filter_expression;

pub use crate::json_tag::JsonTag;
pub use crate::json_node::JsonNode;