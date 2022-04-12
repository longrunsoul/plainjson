//! JSONPath implementation except filter expression(currently filter part of JSONPath not implemented yet).

use anyhow::{bail, Result};
use std::{io::Read, str::FromStr};

use crate::json_node::*;
use crate::peekable_codepoints::*;
// use crate::filter_expression::*;

/// JSONPath part fragment types.<br>
/// There are mainly 3 types of JSONPath part fragment: path name, array element selector, filter.<br>
/// For example, in below JSONPath instance:<br>
/// ```text
/// $.store[3].book[?(@.price < 10)]
/// ```
/// "$", "store", "book" are path names.<br>
/// "\[3\]" is an array element selector.<br>
/// "\[?(@.price < 10)\]" is a filter.
#[derive(Debug, PartialEq)]
pub enum PartFragType {
    None,
    RootPathName,
    CurrentPathName,
    DotNotationPathName,
    BracketNotationPathName,
    ElementSelector,
    Filter,
}

impl PartFragType {
    /// Identify fragment type of following JSONPath part from a PeekableCodePoints instance.
    pub(crate) fn identify_frag<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self>
    where
        R: Read,
    {
        let frag_type = match peekable_cp.peek_char(0)? {
            None => PartFragType::None,
            Some(c) => match c {
                '.' => PartFragType::None,
                '$' => PartFragType::RootPathName,
                '@' => PartFragType::CurrentPathName,
                '[' => match peekable_cp.peek_char(1)? {
                    None => bail!("unexpected end: {}", peekable_cp.peek(1)?),
                    Some(c) => match c {
                        '\'' => PartFragType::BracketNotationPathName,
                        '0'..='9' | '-' | ':' | '*' => PartFragType::ElementSelector,
                        '?' => PartFragType::Filter,
                        _ => bail!(
                            "unrecognized json path part fragment: {}...",
                            peekable_cp.peek(2)?
                        ),
                    },
                },
                _ => PartFragType::DotNotationPathName,
            },
        };

        Ok(frag_type)
    }
}

/// Array element selector. There are 4 types of array element selector:
/// - All: selects all elements of array
/// - Single: select 1 single element of array
/// - Range: selects a range of elements of array
/// - Multiple: selects several distinct elements of array
#[derive(Debug, PartialEq)]
pub enum ArrayElementSelector {
    All,
    Single(usize),
    Range(Option<i32>, Option<i32>),
    Multiple(Vec<usize>),
}

impl ArrayElementSelector {
    /// Trim square brackets from both sides of a string if any.
    fn trim_brackets(elem_selector_str: String) -> String {
        let inner_str = if elem_selector_str.starts_with('[') && elem_selector_str.ends_with(']') {
            elem_selector_str
                .chars()
                .skip(1)
                .take(elem_selector_str.chars().count() - 2)
                .collect()
        } else {
            elem_selector_str
        };
        inner_str
    }

    /// Parse single-type or all-type array element selector from a string.
    fn parse_single_or_all(elem_selector_str: String) -> Result<Self> {
        let index_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        if index_str == "*" {
            return Ok(ArrayElementSelector::All);
        }

        let index = usize::from_str(&index_str)?;
        Ok(ArrayElementSelector::Single(index))
    }

    /// Parse range-type array element selector from a string.
    fn parse_range(elem_selector_str: String) -> Result<Self> {
        let range_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        let colon_index = range_str.chars().position(|c| c == ':').unwrap();
        let range_left_str: String = range_str.chars().take(colon_index).collect();
        let range_right_str: String = range_str.chars().skip(colon_index + 1).collect();

        let range_left = if range_left_str.is_empty() {
            None
        } else {
            Some(i32::from_str(&range_left_str)?)
        };
        let range_right = if range_right_str.is_empty() {
            None
        } else {
            Some(i32::from_str(&range_right_str)?)
        };
        Ok(ArrayElementSelector::Range(range_left, range_right))
    }

    /// Parse multiple-type array element selector from a string.
    fn parse_multiple(elem_selector_str: String) -> Result<Self> {
        let mut indexes = Vec::new();

        let mut number_str = String::new();
        let num_list_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        for c in num_list_str.chars() {
            if c == ',' {
                let i = usize::from_str(&number_str)?;
                indexes.push(i);

                number_str = String::new();
            } else {
                if c.is_whitespace() {
                    continue;
                }

                number_str.push(c);
            }
        }
        let i = usize::from_str(&number_str)?;
        indexes.push(i);

        Ok(ArrayElementSelector::Multiple(indexes))
    }

    /// Parse an array element selector from an instance of PeekableCodePoints.
    pub(crate) fn parse<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self>
    where
        R: Read,
    {
        let mut i = 0;
        let mut has_comma = false;
        let mut has_colon = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)?),
                Some(c) => match c {
                    ']' => break,
                    ',' => has_comma = true,
                    ':' => has_colon = true,
                    '0'..='9' | '-' | '*' => (),

                    '[' => (),
                    c if c.is_whitespace() => (),

                    _ => bail!(
                        "unrecognized array element selector: {}...",
                        peekable_cp.peek(i + 1)?
                    ),
                },
            }

            i += 1;
        }

        let elem_selector_str = peekable_cp.pop(i + 1)?;
        let elem_selector = if has_comma {
            ArrayElementSelector::parse_multiple(elem_selector_str)?
        } else if has_colon {
            ArrayElementSelector::parse_range(elem_selector_str)?
        } else {
            ArrayElementSelector::parse_single_or_all(elem_selector_str)?
        };

        Ok(elem_selector)
    }
}

/// JSONPath part, composed of 3 parts: a path name, an array element selector, and a filter.<br>
/// For example, in below JSONPath:
/// ```text
/// $.store[3].book[?(@.price < 10)]
/// ```
/// ```$```, ```store[3]```, ```book[?(@.price < 10)]``` are 3 JSONPath parts.<br>
/// ```$``` is a JSONPath part composed of only a path name ```$```.<br>
/// ```store[3]``` is a JSONPath part composed of a path name ```store``` and an array element selector ```[3]```.<br>
/// ```book[?(@.price < 10)]``` is a JSONPath part composed of a path name ```book``` and a filter ```[?(@.price < 10)]```.
#[derive(Debug, PartialEq)]
pub struct JsonPathPart {
    pub path_name: String,
    pub elem_selector: Option<ArrayElementSelector>,
    // pub filter: Option<FilterExpression>,
}

impl JsonPathPart {
    /// Create a JSONPath part from a path name, an optional array element selector, and an optional filter expression.
    fn new(
        path_name: &str,
        elem_selector: Option<ArrayElementSelector>, /*, filter: Option<FilterExpression>*/
    ) -> Self {
        JsonPathPart {
            path_name: String::from(path_name),
            elem_selector,
            // filter,
        }
    }

    /// Parse dot notation type path name from an instance of PeekableCodePoints.
    pub(crate) fn parse_dot_notation_path_name<R>(
        peekable_cp: &mut PeekableCodePoints<R>,
    ) -> Result<String>
    where
        R: Read,
    {
        let mut i = 0;
        let mut is_escape = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => break,
                Some(c) => match c {
                    '\\' => {
                        is_escape = true;

                        i += 1;
                        continue;
                    }
                    '.' | '[' if !is_escape => break,
                    _ => (),
                },
            }

            is_escape = false;
            i += 1;
        }
        if 0 == i {
            bail!("empty json path part fragment");
        }

        let path_name = peekable_cp.pop(i)?;
        Ok(path_name)
    }

    /// Parse bracket notation type path name from an instance of PeekableCodePoints.
    pub(crate) fn parse_bracket_notation_path_name<R>(
        peekable_cp: &mut PeekableCodePoints<R>,
    ) -> Result<String>
    where
        R: Read,
    {
        let mut i = 0;
        let mut is_escape = false;
        let mut in_quote = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)?),
                Some(c) => match c {
                    '\\' => {
                        is_escape = true;

                        i += 1;
                        continue;
                    }
                    '\'' if !is_escape => {
                        in_quote = !in_quote;
                        if !in_quote {
                            match peekable_cp.peek_char(i + 1)? {
                                None => bail!("unexpected end: {}", peekable_cp.peek(i + 1)?),
                                Some(c) => match c {
                                    ']' => break,
                                    _ => bail!(
                                        "expecting ] at the end: {}",
                                        peekable_cp.peek(i + 2)?
                                    ),
                                },
                            }
                        }
                    }
                    _ => (),
                },
            }

            i += 1;
            is_escape = false;
        }

        let mut path_name_w_bracket = peekable_cp.pop(i + 2)?;
        if path_name_w_bracket.starts_with("['") && path_name_w_bracket.ends_with("']") {
            path_name_w_bracket = path_name_w_bracket.chars().skip(2).take(i - 2).collect();
        }

        Ok(path_name_w_bracket)
    }

    /// Parse 1 JSONPath part from an instance of PeekableCodePoints.
    /// Note: filter parsing is not implemented yet.
    pub(crate) fn parse_next<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
    where
        R: Read,
    {
        let path_name;
        let mut elem_selector = None;
        // let mut filter = None;

        let frag_type = PartFragType::identify_frag(peekable_cp)?;
        match frag_type {
            PartFragType::None => return Ok(None),
            PartFragType::RootPathName => {
                path_name = String::from("$");
                peekable_cp.skip(1)?;
            }
            PartFragType::CurrentPathName => {
                path_name = String::from("@");
                peekable_cp.skip(1)?;
            }
            PartFragType::DotNotationPathName => {
                path_name = JsonPathPart::parse_dot_notation_path_name(peekable_cp)?
            }
            PartFragType::BracketNotationPathName => {
                path_name = JsonPathPart::parse_bracket_notation_path_name(peekable_cp)?
            }
            _ => bail!("unexpected json path part type: {:?}", frag_type),
        }

        // TODO: IMPLEMENT PARSING OF FILTER EXPRESSION
        let next_frag_type = PartFragType::identify_frag(peekable_cp)?;
        match next_frag_type {
            PartFragType::ElementSelector => {
                elem_selector = Some(ArrayElementSelector::parse(peekable_cp)?)
            }
            PartFragType::Filter => {}
            _ => (),
        }

        let last_frag_type = PartFragType::identify_frag(peekable_cp)?;
        if last_frag_type == PartFragType::Filter {
            todo!("implement json path filter");
        }

        let part = JsonPathPart::new(&path_name, elem_selector /*, filter*/);
        Ok(Some(part))
    }
}

/// Get selected elements out of a mut slice reference as mut reference by indexes.
fn get_mut_by_indexes<'a, T>(vec: &'a mut [T], indexes: &[usize]) -> Vec<&'a mut T> {
    let mut results = Vec::new();

    let mut i = 0;
    let mut iter = vec.iter_mut();
    loop {
        let elem = iter.next();
        if elem.is_none() {
            break;
        }

        let elem = elem.unwrap();
        if indexes.contains(&i) {
            results.push(elem);
        }

        i += 1;
    }

    results
}

/// Get selected elements out of a mut slice reference as mut reference by index range.
fn get_mut_by_index_range<T>(vec: &mut [T], start: usize, end: usize) -> Vec<&mut T> {
    let mut results = Vec::new();

    let mut i = 0;
    let mut iter = vec.iter_mut();
    loop {
        let elem = iter.next();
        if elem.is_none() {
            break;
        }
        if i < start {
            i += 1;
            continue;
        }
        if i >= end {
            break;
        }

        let elem = elem.unwrap();
        results.push(elem);

        i += 1;
    }

    results
}

/// JSONPath, composed of JSONPath parts.
#[derive(Debug, PartialEq)]
pub struct JsonPath {
    pub parts: Vec<JsonPathPart>,
}

impl JsonPath {
    /// Create JsonPath from a list of JSONPath parts.
    fn new(parts: Vec<JsonPathPart>) -> Self {
        JsonPath { parts }
    }

    /// Parse JsonPath from a string representation of it.
    pub fn parse(path_str: &str) -> Result<Self> {
        let mut path_parts = Vec::new();
        let mut peekable_cp = PeekableCodePoints::new(path_str.as_bytes());
        loop {
            let part = JsonPathPart::parse_next(&mut peekable_cp)?;
            if part.is_none() {
                break;
            }

            path_parts.push(part.unwrap());

            if Some('.') == peekable_cp.peek_char(0)? {
                peekable_cp.skip(1)?;
            }
        }

        let json_path = JsonPath::new(path_parts);
        Ok(json_path)
    }

    /// Evaluate JSONPath to a list of mutable JsonNode.
    fn evaluate_json_path<'a>(&self, json_node: &'a mut JsonNode) -> Result<Vec<&'a mut JsonNode>> {
        let mut current = vec![json_node];
        for path_part in &self.parts {
            match path_part.path_name.as_str() {
                "$" => (),
                "@" => (),
                pn => {
                    let mut next = Vec::new();
                    for c in current {
                        // only handle object notation
                        if let JsonNode::Object(pl) = c {
                            let prop_index = pl.iter().position(|x| x.name == pn);

                            // ignore if not found
                            if let Some(prop_index) = prop_index {
                                let n = &mut pl[prop_index].value;
                                next.push(n);
                            }
                        }
                    }

                    current = next;
                }
            }

            match &path_part.elem_selector {
                None => (),
                Some(es) => {
                    let mut next = Vec::new();
                    for c in current {
                        match c {
                            JsonNode::Array(arr) => {
                                let arr_len = arr.len();
                                match es {
                                    ArrayElementSelector::Single(i) => {
                                        if *i < arr_len {
                                            next.push(&mut arr[*i]);
                                        }
                                    }
                                    ArrayElementSelector::Multiple(il) => {
                                        let mut selected = get_mut_by_indexes(arr, il);
                                        next.append(&mut selected);
                                    }
                                    ArrayElementSelector::Range(s, e) => {
                                        match s {
                                            None => {
                                                match e {
                                                    None => next.extend(arr.iter_mut()),
                                                    Some(e) if *e < 0 => bail!("array element selector end index must not be negative: [:{}]", e),
                                                    Some(e) if *e >= 0 => {
                                                        let mut selected = get_mut_by_index_range(arr, 0, *e as usize);
                                                        next.append(&mut selected);
                                                    }
                                                    _ => bail!("range element selector unreachable code reached!"),
                                                }
                                            }
                                            Some(s) if *s < 0 => {
                                                match e {
                                                    None => {
                                                        let mut selected = get_mut_by_index_range(arr, ((arr_len as i32) + *s) as usize, arr_len);
                                                        next.append(&mut selected);
                                                    }
                                                    Some(e) => bail!("array element selector start index must not be negative when end index specified: [{}:{}]", s, e),
                                                }
                                            }
                                            Some(s) if *s >= 0 => {
                                                match e {
                                                    None => {
                                                        let mut selected = get_mut_by_index_range(arr, *s as usize, arr_len);
                                                        next.append(&mut selected);
                                                    }
                                                    Some(e) if *e < 0 => bail!("array element selector end index must not be negative: [{}:{}]", s, e),
                                                    Some(e) if *e >= 0 => {
                                                        let mut selected = get_mut_by_index_range(arr, *s as usize, *e as usize);
                                                        next.append(&mut selected);
                                                    }
                                                    _ => bail!("range element selector unreachable code reached!"),
                                                }
                                            }
                                            _ => bail!("range element selector unreachable code reached!"),
                                        }
                                    }
                                    ArrayElementSelector::All => {
                                        let mut selected = get_mut_by_index_range(arr, 0, arr_len);
                                        next.append(&mut selected);
                                    }
                                }
                            }
                            _ => bail!("element selector must be applied on array notation, {} is not array", path_part.path_name)
                        }
                    }

                    current = next;
                }
            }
        }

        Ok(current)
    }

    /// Get a list of raw JsonNode by a JsonPath.
    pub(crate) fn json_path_get_raw<'a>(
        &self,
        json_node: &'a mut JsonNode,
    ) -> Result<Vec<&'a JsonNode>> {
        let mut result = Vec::new();
        let nodes = self.evaluate_json_path(json_node)?;
        for n in nodes {
            result.push(&*n);
        }

        Ok(result)
    }

    /// Get a list of number value by a JsonPath. The nodes selected by JsonPath must be of number type.
    pub(crate) fn json_path_get_number(&self, json_node: &mut JsonNode) -> Result<Vec<f64>> {
        let mut numbers = Vec::new();
        let selected = self.json_path_get_raw(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainNumber(num) => numbers.push(*num),
                _ => bail!("expecting number, but found: {:?}", n),
            }
        }

        Ok(numbers)
    }

    /// Get a list of bool value by a JsonPath. The nodes selected by JsonPath must be of bool type.
    pub(crate) fn json_path_get_bool(&self, json_node: &mut JsonNode) -> Result<Vec<bool>> {
        let mut bvalues = Vec::new();
        let selected = self.json_path_get_raw(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainBoolean(b) => bvalues.push(*b),
                _ => bail!("expecting bool, but found: {:?}", n),
            }
        }

        Ok(bvalues)
    }

    /// Get a list of string value by a JsonPath. The nodes selected by JsonPath must be of string type.
    pub(crate) fn json_path_get_str(&self, json_node: &mut JsonNode) -> Result<Vec<String>> {
        let mut strs = Vec::new();
        let selected = self.json_path_get_raw(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainString(s) => strs.push(s.clone()),
                _ => bail!("expecting string, but found: {:?}", n),
            }
        }

        Ok(strs)
    }

    /// Set the nodes selected by JsonPath to null value.
    pub(crate) fn json_path_set_null(&self, json_node: &mut JsonNode) -> Result<()> {
        self.json_path_set_raw(json_node, &JsonNode::PlainNull)
    }

    /// Set the nodes selected by JsonPath to a value of specified number.
    pub(crate) fn json_path_set_number(&self, json_node: &mut JsonNode, value: f64) -> Result<()> {
        self.json_path_set_raw(json_node, &JsonNode::PlainNumber(value))
    }

    /// Set the nodes selected by JsonPath to a value of specified bool.
    pub(crate) fn json_path_set_bool(&self, json_node: &mut JsonNode, value: bool) -> Result<()> {
        self.json_path_set_raw(json_node, &JsonNode::PlainBoolean(value))
    }

    /// Set the nodes selected by JsonPath to a value of specified string.
    pub(crate) fn json_path_set_str(&self, json_node: &mut JsonNode, value: &str) -> Result<()> {
        self.json_path_set_raw(json_node, &JsonNode::PlainString(String::from(value)))
    }

    /// Set the nodes selected by JsonPath to a value of specified raw JsonNode.
    pub(crate) fn json_path_set_raw(
        &self,
        json_node: &mut JsonNode,
        value: &JsonNode,
    ) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = value.clone();
        }

        Ok(())
    }
}

impl JsonNode {
    /// Get a number value of the node selected by specified JSONPath.
    pub fn get_number(&mut self, json_path: &str) -> Result<Option<f64>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_number(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    /// Get a bool value of the node selected by specified JSONPath.
    pub fn get_bool(&mut self, json_path: &str) -> Result<Option<bool>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_bool(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    /// Get a string value of the node selected by specified JSONPath.
    pub fn get_str(&mut self, json_path: &str) -> Result<Option<String>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_str(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    /// Get the raw JsonNode of the node selected by specified JSONPath.
    pub fn get_raw(&mut self, json_path: &str) -> Result<Option<&JsonNode>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_raw(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    /// Set the value of nodes selected by specified JSONPath to null.
    pub fn set_null(&mut self, json_path: &str) -> Result<()> {
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_null(self)?;
        Ok(())
    }

    /// Set the value of nodes selected by specified JSONPath to specified number.
    pub fn set_number(&mut self, json_path: &str, value: f64) -> Result<()> {
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_number(self, value)?;
        Ok(())
    }

    /// Set the value of nodes selected by specified JSONPath to specified bool.
    pub fn set_bool(&mut self, json_path: &str, value: bool) -> Result<()> {
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_bool(self, value)?;
        Ok(())
    }

    /// Set the value of nodes selected by specified JSONPath to specified string.
    pub fn set_str(&mut self, json_path: &str, value: &str) -> Result<()> {
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_str(self, value)?;
        Ok(())
    }

    /// Set the value of nodes selected by specified JSONPath to specified raw JsonNode.
    pub fn set_raw(&mut self, json_path: &str, value: &JsonNode) -> Result<()> {
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_raw(self, value)?;
        Ok(())
    }
}

#[cfg(test)]
mod json_path_tests {
    use super::*;

    /// Test dot-notation-type JSONPath(without filter) parsing.
    #[test]
    fn test_no_filter_dot_notation() -> Result<()> {
        let json_path_str = r#"$[-1:].store[:3].bicycle[0, 13].color[*]"#;
        let json_path = JsonPath::parse(json_path_str)?;
        assert_eq!(
            json_path,
            JsonPath::new(vec![
                JsonPathPart::new(
                    "$",
                    Some(ArrayElementSelector::Range(Some(-1), None)) /*, None*/
                ),
                JsonPathPart::new(
                    "store",
                    Some(ArrayElementSelector::Range(None, Some(3))) /*, None*/
                ),
                JsonPathPart::new(
                    "bicycle",
                    Some(ArrayElementSelector::Multiple(vec![0, 13])) /*, None*/
                ),
                JsonPathPart::new("color", Some(ArrayElementSelector::All) /*, None*/),
            ])
        );

        Ok(())
    }

    /// Test bracket-notation-type JSONPath(without filter) parsing.
    #[test]
    fn test_no_filter_bracket_notation() -> Result<()> {
        let json_path_str = r#"$[-1:]['store'][:3]['bicycle'][0, 13]['color'][*]"#;
        let json_path = JsonPath::parse(json_path_str)?;
        assert_eq!(
            json_path,
            JsonPath::new(vec![
                JsonPathPart::new(
                    "$",
                    Some(ArrayElementSelector::Range(Some(-1), None)) /*, None*/
                ),
                JsonPathPart::new(
                    "store",
                    Some(ArrayElementSelector::Range(None, Some(3))) /*, None*/
                ),
                JsonPathPart::new(
                    "bicycle",
                    Some(ArrayElementSelector::Multiple(vec![0, 13])) /*, None*/
                ),
                JsonPathPart::new("color", Some(ArrayElementSelector::All) /*, None*/),
            ])
        );

        Ok(())
    }

    /// Test JSONPath evaluation of array elements.
    #[test]
    fn test_json_path_get_array() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = r#"$.array[1]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(selected, vec![&JsonNode::PlainString(String::from("b"))]);

        Ok(())
    }

    /// Test JSONPath evaluation of object property value.
    #[test]
    fn test_json_path_get_object() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = r#"$.object.prop"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(
            selected,
            vec![&JsonNode::PlainString(String::from(r#"{true]"#))]
        );

        Ok(())
    }

    /// Test JSONPath evaluation, a little complex one.
    #[test]
    fn test_json_path_get_complex() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = r#"$.object.nested[-4:]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainBoolean(false),
                &JsonNode::PlainNumber(3f64),
                &JsonNode::PlainString(String::from("yes")),
                &JsonNode::PlainString(String::from("no")),
            ]
        );

        Ok(())
    }

    /// Test bracket-notation-type JSONPath evaluation.
    #[test]
    fn test_json_path_get_bracket_notation() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = r#"$['object']['nested'][1, 3]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainBoolean(false),
                &JsonNode::PlainString(String::from("yes")),
            ]
        );

        Ok(())
    }

    /// Test value assignment by JSONPath, the simple one.
    #[test]
    fn test_json_path_set_simple() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = "$.simple";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_bool(&mut json_node, true)?;

        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(selected, vec![&JsonNode::PlainBoolean(true)]);

        Ok(())
    }

    /// Test value assignment by JSONPath, the array.
    #[test]
    fn test_json_path_set_simple_array() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = "$.array[*]";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_str(&mut json_node, "yes")?;

        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainString(String::from("yes")),
                &JsonNode::PlainString(String::from("yes")),
                &JsonNode::PlainString(String::from("yes")),
            ]
        );

        Ok(())
    }

    /// Test value assignment by JSONPath, a little complex one.
    #[test]
    fn test_json_path_set_complex() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let mut json_node = JsonNode::parse_single_node(json.as_bytes())?;

        let json_path = "$.array";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_raw(
            &mut json_node,
            &JsonNode::Object(vec![
                JsonObjProp::new(String::from("hello"), JsonNode::PlainNumber(1f64)),
                JsonObjProp::new(String::from("world"), JsonNode::PlainNumber(2f64)),
                JsonObjProp::new(String::from("love"), JsonNode::PlainNumber(3f64)),
            ]),
        )?;

        let selected = json_path.json_path_get_raw(&mut json_node)?;
        assert_eq!(
            selected,
            vec![&JsonNode::Object(vec![
                JsonObjProp::new(String::from("hello"), JsonNode::PlainNumber(1f64)),
                JsonObjProp::new(String::from("world"), JsonNode::PlainNumber(2f64)),
                JsonObjProp::new(String::from("love"), JsonNode::PlainNumber(3f64)),
            ])]
        );

        Ok(())
    }
}
