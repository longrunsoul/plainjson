use std::{
    io::Read,
    str::FromStr,
};
use anyhow::{
    Result,
    bail,
};

use crate::peekable_codepoints::*;
use crate::filter_expression::*;
use crate::json_node::*;

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
    pub(crate) fn identify_frag<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self>
        where R: Read {
        let frag_type =
            match peekable_cp.peek_char(0)? {
                None => PartFragType::None,
                Some(c) => {
                    match c {
                        '.' => PartFragType::None,
                        '$' => PartFragType::RootPathName,
                        '@' => PartFragType::CurrentPathName,
                        '[' => {
                            match peekable_cp.peek_char(1)? {
                                None => bail!("unexpected end: {}", peekable_cp.peek(1)?),
                                Some(c) => {
                                    match c {
                                        '\'' => PartFragType::BracketNotationPathName,
                                        '0'..='9' | '-' | ':' | '*' => PartFragType::ElementSelector,
                                        '?' => PartFragType::Filter,
                                        _ => bail!("unrecognized json path part fragment: {}...", peekable_cp.peek(2)?)
                                    }
                                }
                            }
                        }
                        _ => PartFragType::DotNotationPathName,
                    }
                }
            };

        Ok(frag_type)
    }
}

#[derive(Debug, PartialEq)]
pub enum ArrayElementSelector {
    All,
    Single(usize),
    Range(Option<i32>, Option<i32>),
    Multiple(Vec<usize>),
}

impl ArrayElementSelector {
    fn trim_brackets(elem_selector_str: String) -> String {
        let inner_str =
            if elem_selector_str.starts_with("[") && elem_selector_str.ends_with("]") {
                elem_selector_str.chars().skip(1).take(elem_selector_str.chars().count() - 2).collect()
            } else {
                elem_selector_str
            };
        inner_str
    }

    fn parse_single_or_all(elem_selector_str: String) -> Result<Self> {
        let index_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        if index_str == "*" {
            return Ok(ArrayElementSelector::All);
        }

        let index = usize::from_str(&index_str)?;
        Ok(ArrayElementSelector::Single(index))
    }

    fn parse_range(elem_selector_str: String) -> Result<Self> {
        let range_str = ArrayElementSelector::trim_brackets(elem_selector_str);
        let colon_index = range_str.chars().position(|c| c == ':').unwrap();
        let range_left_str: String = range_str.chars().take(colon_index).collect();
        let range_right_str: String = range_str.chars().skip(colon_index + 1).collect();

        let range_left =
            if range_left_str.is_empty() {
                None
            } else {
                Some(i32::from_str(&range_left_str)?)
            };
        let range_right =
            if range_right_str.is_empty() {
                None
            } else {
                Some(i32::from_str(&range_right_str)?)
            };
        Ok(ArrayElementSelector::Range(range_left, range_right))
    }

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

    pub(crate) fn parse<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Self>
        where R: Read {
        let mut i = 0;
        let mut has_comma = false;
        let mut has_colon = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)?),
                Some(c) => {
                    match c {
                        ']' => break,
                        ',' => has_comma = true,
                        ':' => has_colon = true,
                        '0'..='9' | '-' | '*' => (),

                        '[' => (),
                        c if c.is_whitespace() => (),

                        _ => bail!("unrecognized array element selector: {}...", peekable_cp.peek(i+1)?),
                    }
                }
            }

            i += 1;
        }

        let elem_selector_str = peekable_cp.pop(i + 1)?;
        let elem_selector =
            if has_comma {
                ArrayElementSelector::parse_multiple(elem_selector_str)?
            } else if has_colon {
                ArrayElementSelector::parse_range(elem_selector_str)?
            } else {
                ArrayElementSelector::parse_single_or_all(elem_selector_str)?
            };

        Ok(elem_selector)
    }
}

#[derive(Debug, PartialEq)]
pub struct JsonPathPart {
    pub path_name: String,
    pub elem_selector: Option<ArrayElementSelector>,
    pub filter: Option<FilterExpression>,
}

impl JsonPathPart {
    fn new(path_name: &str, elem_selector: Option<ArrayElementSelector>, filter: Option<FilterExpression>) -> Self {
        JsonPathPart {
            path_name: String::from(path_name),
            elem_selector,
            filter,
        }
    }

    pub(crate) fn parse_dot_notation_path_name<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<String>
        where R: Read {
        let mut i = 0;
        let mut is_escape = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => break,
                Some(c) => {
                    match c {
                        '\\' => {
                            is_escape = true;

                            i += 1;
                            continue;
                        }
                        '.' | '[' if !is_escape => break,
                        _ => (),
                    }
                }
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

    pub(crate) fn parse_bracket_notation_path_name<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<String>
        where R: Read {
        let mut i = 0;
        let mut is_escape = false;
        let mut in_quote = false;
        loop {
            match peekable_cp.peek_char(i)? {
                None => bail!("unexpected end: {}", peekable_cp.peek(i)?),
                Some(c) => {
                    match c {
                        '\\' => {
                            is_escape = true;

                            i += 1;
                            continue;
                        }
                        '\'' if !is_escape => {
                            in_quote = !in_quote;
                            if false == in_quote {
                                match peekable_cp.peek_char(i + 1)? {
                                    None => bail!("unexpected end: {}", peekable_cp.peek(i + 1)?),
                                    Some(c) => {
                                        match c {
                                            ']' => break,
                                            _ => bail!("expecting ] at the end: {}", peekable_cp.peek(i + 2)?),
                                        }
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
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

    pub(crate) fn parse_next<R>(peekable_cp: &mut PeekableCodePoints<R>) -> Result<Option<Self>>
        where R: Read {
        let path_name;
        let mut elem_selector = None;
        let mut filter = None;

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
            PartFragType::DotNotationPathName => path_name = JsonPathPart::parse_dot_notation_path_name(peekable_cp)?,
            PartFragType::BracketNotationPathName => path_name = JsonPathPart::parse_bracket_notation_path_name(peekable_cp)?,
            _ => bail!("unexpected json path part type: {:?}", frag_type),
        }

        // TODO: IMPLEMENT PARSING OF FILTER EXPRESSION
        let next_frag_type = PartFragType::identify_frag(peekable_cp)?;
        match next_frag_type {
            PartFragType::ElementSelector => elem_selector = Some(ArrayElementSelector::parse(peekable_cp)?),
            PartFragType::Filter => {}
            _ => (),
        }

        let last_frag_type = PartFragType::identify_frag(peekable_cp)?;
        match last_frag_type {
            PartFragType::Filter => {}
            _ => (),
        }

        let part = JsonPathPart::new(&path_name, elem_selector, filter);
        Ok(Some(part))
    }
}

fn get_mut_by_indexes<'a, T>(vec: &'a mut Vec<T>, indexes: &Vec<usize>) -> Vec<&'a mut T> {
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

fn get_mut_by_index_range<T>(vec: &mut Vec<T>, start: usize, end: usize) -> Vec<&mut T> {
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

#[derive(Debug, PartialEq)]
pub struct JsonPath {
    pub parts: Vec<JsonPathPart>,
}

impl JsonPath {
    fn new(parts: Vec<JsonPathPart>) -> Self {
        JsonPath {
            parts
        }
    }

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

    fn evaluate_json_path<'a>(&self, json_node: &'a mut JsonNode) -> Result<Vec<&'a mut JsonNode>> {
        let mut current = vec![json_node];
        for path_part in &self.parts {
            match path_part.path_name.as_str() {
                "$" => (),
                "@" => (),
                pn => {
                    let mut next = Vec::new();
                    for c in current {
                        match c {
                            JsonNode::Object(pl) => {
                                let prop_index = pl.iter().position(|x| x.name == pn);

                                // ignore if not found
                                if !prop_index.is_none() {
                                    let n = &mut pl[prop_index.unwrap()].value;
                                    next.push(n);
                                }
                            }

                            // ignore when applied on non-object notation
                            _ => (),
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
                                match es {
                                    ArrayElementSelector::Single(i) => {
                                        if *i < arr.len() {
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
                                                        let mut selected = get_mut_by_index_range(arr, ((arr.len() as i32) + *s) as usize, arr.len());
                                                        next.append(&mut selected);
                                                    }
                                                    Some(e) => bail!("array element selector start index must not be negative when end index specified: [{}:{}]", s, e),
                                                }
                                            }
                                            Some(s) if *s >= 0 => {
                                                match e {
                                                    None => {
                                                        let mut selected = get_mut_by_index_range(arr, *s as usize, arr.len());
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
                                        let mut selected = get_mut_by_index_range(arr, 0, arr.len());
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

    pub(crate) fn json_path_get<'a>(&self, json_node: &'a mut JsonNode) -> Result<Vec<&'a JsonNode>> {
        let mut result = Vec::new();
        let nodes = self.evaluate_json_path(json_node)?;
        for n in nodes {
            result.push(&*n);
        }

        Ok(result)
    }

    pub(crate) fn json_path_get_number(&self, json_node: &mut JsonNode) -> Result<Vec<f64>> {
        let mut numbers = Vec::new();
        let selected = self.json_path_get(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainNumber(num) => numbers.push(*num),
                _ => bail!("expecting number, but found: {:?}", n),
            }
        }

        Ok(numbers)
    }

    pub(crate) fn json_path_get_bool(&self, json_node: &mut JsonNode) -> Result<Vec<bool>> {
        let mut bvalues = Vec::new();
        let selected = self.json_path_get(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainBoolean(b) => bvalues.push(*b),
                _ => bail!("expecting bool, but found: {:?}", n),
            }
        }

        Ok(bvalues)
    }

    pub(crate) fn json_path_get_str(&self, json_node: &mut JsonNode) -> Result<Vec<String>> {
        let mut strs = Vec::new();
        let selected = self.json_path_get(json_node)?;
        for n in selected {
            match n {
                JsonNode::PlainString(s) => strs.push(s.clone()),
                _ => bail!("expecting string, but found: {:?}", n),
            }
        }

        Ok(strs)
    }

    pub(crate) fn json_path_set_null(&self, json_node: &mut JsonNode) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = JsonNode::PlainNull;
        }

        Ok(())
    }

    pub(crate) fn json_path_set_number(&self, json_node: &mut JsonNode, value: f64) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = JsonNode::PlainNumber(value);
        }

        Ok(())
    }

    pub(crate) fn json_path_set_bool(&self, json_node: &mut JsonNode, value: bool) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = JsonNode::PlainBoolean(value);
        }

        Ok(())
    }

    pub(crate) fn json_path_set_str(&self, json_node: &mut JsonNode, value: &str) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = JsonNode::PlainString(String::from(value));
        }

        Ok(())
    }

    pub(crate) fn json_path_set_complex(&self, json_node: &mut JsonNode, value: &JsonNode) -> Result<()> {
        let selected = self.evaluate_json_path(json_node)?;
        for n in selected {
            *n = value.clone();
        }

        Ok(())
    }
}

impl JsonNode {
    pub fn get_number(&mut self, json_path: &str) -> Result<Option<f64>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_number(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    pub fn get_bool(&mut self, json_path: &str) -> Result<Option<bool>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_bool( self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    pub fn get_str(&mut self, json_path: &str) -> Result<Option<String>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get_str(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }

    pub fn get_raw(&mut self, json_path: &str) -> Result<Option<&JsonNode>> {
        let json_path = JsonPath::parse(json_path)?;
        let mut selected = json_path.json_path_get(self)?;
        if selected.is_empty() {
            return Ok(None);
        }

        let selected = selected.remove(0);
        Ok(Some(selected))
    }
}

#[cfg(test)]
mod json_path_tests {
    use super::*;
    use crate::json_tag::*;

    #[test]
    fn test_no_filter_dot_notation() -> Result<()> {
        let json_path_str = r#"$[-1:].store[:3].bicycle[0, 13].color[*]"#;
        let json_path = JsonPath::parse(json_path_str)?;
        assert_eq!(
            json_path,
            JsonPath::new(
                vec![
                    JsonPathPart::new("$", Some(ArrayElementSelector::Range(Some(-1), None)), None),
                    JsonPathPart::new("store", Some(ArrayElementSelector::Range(None, Some(3))), None),
                    JsonPathPart::new("bicycle", Some(ArrayElementSelector::Multiple(vec![0, 13])), None),
                    JsonPathPart::new("color", Some(ArrayElementSelector::All), None),
                ]
            )
        );

        Ok(())
    }

    #[test]
    fn test_no_filter_bracket_notation() -> Result<()> {
        let json_path_str = r#"$[-1:]['store'][:3]['bicycle'][0, 13]['color'][*]"#;
        let json_path = JsonPath::parse(json_path_str)?;
        assert_eq!(
            json_path,
            JsonPath::new(
                vec![
                    JsonPathPart::new("$", Some(ArrayElementSelector::Range(Some(-1), None)), None),
                    JsonPathPart::new("store", Some(ArrayElementSelector::Range(None, Some(3))), None),
                    JsonPathPart::new("bicycle", Some(ArrayElementSelector::Multiple(vec![0, 13])), None),
                    JsonPathPart::new("color", Some(ArrayElementSelector::All), None),
                ]
            )
        );

        Ok(())
    }

    #[test]
    fn test_json_path_get_array() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = r#"$.array[1]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainString(String::from("b"))
            ]
        );

        Ok(())
    }

    #[test]
    fn test_json_path_get_object() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]"}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = r#"$.object.prop"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainString(String::from(r#"{true]"#))
            ]
        );

        Ok(())
    }

    #[test]
    fn test_json_path_get_complex() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = r#"$.object.nested[-4:]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get(&mut json_node)?;
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

    #[test]
    fn test_json_path_get_bracket_notation() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = r#"$['object']['nested'][1, 3]"#;
        let json_path = JsonPath::parse(json_path)?;
        let selected = json_path.json_path_get(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainBoolean(false),
                &JsonNode::PlainString(String::from("yes")),
            ]
        );

        Ok(())
    }

    #[test]
    fn test_json_path_set_simple() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = "$.simple";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_bool(&mut json_node, true)?;

        let selected = json_path.json_path_get(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::PlainBoolean(true)
            ]
        );

        Ok(())
    }

    #[test]
    fn test_json_path_set_simple_array() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = "$.array[*]";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_str(&mut json_node, "yes")?;

        let selected = json_path.json_path_get(&mut json_node)?;
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

    #[test]
    fn test_json_path_set_complex() -> Result<()> {
        let json = r#"{"simple": 123, "array": ["a", "b", "c\""], "object": {"prop": "{true]", "nested": [true, false, 3, "yes", "no"]}}"#;
        let json_tag_list = JsonTag::parse(json.as_bytes())?;
        let mut json_node_list = JsonNode::parse(&json_tag_list)?;
        let mut json_node = json_node_list.remove(0);

        let json_path = "$.array";
        let json_path = JsonPath::parse(json_path)?;
        json_path.json_path_set_complex(
            &mut json_node,
            &JsonNode::Object(
                vec![
                    JsonObjProp::new(String::from("hello"), JsonNode::PlainNumber(1f64)),
                    JsonObjProp::new(String::from("world"), JsonNode::PlainNumber(2f64)),
                    JsonObjProp::new(String::from("love"), JsonNode::PlainNumber(3f64)),
                ]
            )
        )?;

        let selected = json_path.json_path_get(&mut json_node)?;
        assert_eq!(
            selected,
            vec![
                &JsonNode::Object(
                    vec![
                        JsonObjProp::new(String::from("hello"), JsonNode::PlainNumber(1f64)),
                        JsonObjProp::new(String::from("world"), JsonNode::PlainNumber(2f64)),
                        JsonObjProp::new(String::from("love"), JsonNode::PlainNumber(3f64)),
                    ]
                )
            ]
        );

        Ok(())
    }
}