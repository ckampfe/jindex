#![forbid(unsafe_code)]

pub mod path_value_sink;

use anyhow::{anyhow, Result};
use path_value_sink::PathValueSink;
use serde::Serialize;

const DEFAULT_PATH_COMPONENTS_CAPACITY: usize = std::mem::size_of::<usize>();

/// Enumerate the paths through a JSON document
///
/// `jindex` will call `sink.handle_pathvalue` every time it reaches a new
/// node of the json document, passing it a [PathValue]
/// containing the path to reach that node (as `Vec` of [PathComponent]),
/// and the value ([serde_json::Value]) at that node.
pub fn jindex<S: PathValueSink>(sink: &mut S, json: &serde_json::Value) -> Result<()> {
    if !json.is_object() && !json.is_array() {
        return Err(anyhow!(
            "input value must be either a JSON array or JSON object, got: {}",
            json
        ));
    }

    let root_pathvalue = PathValue::new(json, Vec::new());

    let mut traversal_stack: Vec<PathValue> = vec![root_pathvalue];

    while let Some(pathvalue) = traversal_stack.pop() {
        match pathvalue.value {
            serde_json::Value::Object(object) => {
                traverse_object(&mut traversal_stack, object, &pathvalue);
            }
            serde_json::Value::Array(array) => {
                traverse_array(&mut traversal_stack, array, &pathvalue);
            }
            _terminal_value => (),
        }

        sink.handle_pathvalue(&pathvalue)?;
    }

    Ok(())
}

#[derive(Clone, Debug, Serialize)]
pub struct PathValue<'a> {
    pub path_components: Vec<PathComponent<'a>>,
    pub value: &'a serde_json::Value,
}

impl<'a> PathValue<'a> {
    fn new(value: &'a serde_json::Value, path_components: Vec<PathComponent<'a>>) -> Self {
        Self {
            value,
            path_components,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(untagged)]
pub enum PathComponent<'a> {
    Identifier(&'a str),
    NonIdentifier(&'a str),
    Index(usize),
}

fn traverse_object<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    object: &'a serde_json::Map<String, serde_json::Value>,
    pathvalue: &PathValue<'a>,
) {
    traversal_stack.extend(object.iter().map(|(k, v)| {
        let mut cloned = Vec::with_capacity(DEFAULT_PATH_COMPONENTS_CAPACITY);

        cloned.clone_from(&pathvalue.path_components);

        let component = if is_identifier(k) {
            PathComponent::Identifier(k)
        } else {
            PathComponent::NonIdentifier(k)
        };

        cloned.push(component);

        PathValue::new(v, cloned)
    }))
}

fn traverse_array<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    array: &'a [serde_json::Value],
    pathvalue: &PathValue<'a>,
) {
    traversal_stack.extend(array.iter().enumerate().map(|(i, v)| {
        let mut cloned = Vec::with_capacity(DEFAULT_PATH_COMPONENTS_CAPACITY);

        cloned.clone_from(&pathvalue.path_components);

        cloned.push(PathComponent::Index(i));

        PathValue::new(v, cloned)
    }))
}

fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();

    chars.next().map_or(false, unicode_ident::is_xid_start)
        && chars.all(unicode_ident::is_xid_continue)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod gron {
        use super::*;
        use crate::path_value_sink::{GronWriter, GronWriterOptions};

        #[test]
        fn one() {
            let expected = std::fs::read_to_string("fixtures/one.gron").unwrap();

            let mut expected: Vec<&str> = expected
                .split('\n')
                .filter(|line| !line.is_empty())
                .collect();
            expected.sort_unstable();

            let input = std::fs::read_to_string("fixtures/one.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();

            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);

            jindex(&mut sink, &parsed).unwrap();
            let challenge = String::from_utf8(challenge).unwrap();
            let challenge = challenge.trim();

            let mut challenge: Vec<&str> = challenge.split('\n').collect();
            challenge.sort_unstable();

            assert_eq!(expected, challenge);
        }

        #[test]
        fn two() {
            let expected = std::fs::read_to_string("fixtures/two.gron").unwrap();

            let mut expected: Vec<&str> = expected
                .split('\n')
                .filter(|line| !line.is_empty())
                .collect();
            expected.sort_unstable();

            let input = std::fs::read_to_string("fixtures/two.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();
            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);
            jindex(&mut sink, &parsed).unwrap();
            let challenge = String::from_utf8(challenge).unwrap();
            let challenge = challenge.trim();

            let mut challenge: Vec<&str> = challenge.split('\n').collect();
            challenge.sort_unstable();

            assert_eq!(expected, challenge);
        }

        #[test]
        fn three() {
            let expected = std::fs::read_to_string("fixtures/three.gron").unwrap();

            let mut expected: Vec<&str> = expected
                .split('\n')
                .filter(|line| !line.is_empty())
                .collect();
            expected.sort_unstable();

            let input = std::fs::read_to_string("fixtures/three.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();
            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);
            jindex(&mut sink, &parsed).unwrap();
            let challenge = String::from_utf8(challenge).unwrap();
            let challenge = challenge.trim();

            let mut challenge: Vec<&str> = challenge.split('\n').collect();
            challenge.sort_unstable();

            assert_eq!(expected, challenge);
        }

        #[test]
        fn github() {
            let expected = std::fs::read_to_string("fixtures/github.gron").unwrap();

            let mut expected: Vec<&str> = expected
                .split('\n')
                .filter(|line| !line.is_empty())
                .collect();
            expected.sort_unstable();

            let input = std::fs::read_to_string("fixtures/github.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();
            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);
            jindex(&mut sink, &parsed).unwrap();
            let challenge = String::from_utf8(challenge).unwrap();
            let challenge = challenge.trim();

            let mut challenge: Vec<&str> = challenge.split('\n').collect();
            challenge.sort_unstable();

            assert_eq!(expected, challenge);
        }

        #[test]
        fn large_line() {
            let expected = std::fs::read_to_string("fixtures/large-line.gron").unwrap();

            let mut expected: Vec<&str> = expected
                .split('\n')
                .filter(|line| !line.is_empty())
                .collect();
            expected.sort_unstable();

            let input = std::fs::read_to_string("fixtures/large-line.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();
            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);
            jindex(&mut sink, &parsed).unwrap();
            let challenge = String::from_utf8(challenge).unwrap();
            let challenge = challenge.trim();

            let mut challenge: Vec<&str> = challenge.split('\n').collect();
            challenge.sort_unstable();

            assert_eq!(expected, challenge);
        }

        #[test]
        fn big() {
            // 923k is not really that big but this is what gron itself uses
            let input = std::fs::read_to_string("fixtures/big.json").unwrap();

            let parsed = serde_json::from_str(&input).unwrap();

            let mut challenge = Vec::new();
            let options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut challenge, options);
            // simply asserting that we don't panic here
            jindex(&mut sink, &parsed).unwrap();
        }
    }

    mod json_pointer {
        use super::*;
        use crate::path_value_sink::{JSONPointerWriter, JSONPointerWriterOptions};
        use std::collections::HashSet;

        #[test]
        fn simple_document() {
            let v: serde_json::Value = serde_json::json!(
                {
                    "a": 1,
                    "b": 2,
                    "c": ["x", "y", "z"],
                    "d": {"e": {"f": [{}, 9, "g"]}}
                }
            );

            let mut challenge = Vec::new();
            let mut sink = JSONPointerWriter::new(
                &mut challenge,
                JSONPointerWriterOptions {
                    separator: "@@@",
                    only_scalars: false,
                },
            );

            jindex(&mut sink, &v).unwrap();

            let challenge = std::str::from_utf8(&challenge)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>();

            let expected = HashSet::from([
                r#"/a@@@1"#,
                r#"/b@@@2"#,
                r#"/c@@@["x","y","z"]"#,
                r#"/d@@@{"e":{"f":[{},9,"g"]}}"#,
                r#"/c/0@@@"x""#,
                r#"/c/1@@@"y""#,
                r#"/c/2@@@"z""#,
                r#"/d/e@@@{"f":[{},9,"g"]}"#,
                r#"/d/e/f@@@[{},9,"g"]"#,
                r#"/d/e/f/0@@@{}"#,
                r#"/d/e/f/1@@@9"#,
                r#"/d/e/f/2@@@"g""#,
            ]);

            assert_eq!(challenge, expected)
        }

        #[test]
        fn only_terminals() {
            let v: serde_json::Value = serde_json::json!(
                {
                    "a": 1,
                    "b": 2,
                    "c": ["x", "y", "z"],
                    "d": {"e": {"f": [{}, 9, "g", []]}}
                }
            );

            let mut challenge = Vec::new();
            let mut sink = JSONPointerWriter::new(
                &mut challenge,
                JSONPointerWriterOptions {
                    separator: "@@@",
                    only_scalars: true,
                },
            );

            jindex(&mut sink, &v).unwrap();

            let challenge = std::str::from_utf8(&challenge)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>();

            let expected = HashSet::from([
                r#"/a@@@1"#,
                r#"/b@@@2"#,
                r#"/c/0@@@"x""#,
                r#"/c/1@@@"y""#,
                r#"/c/2@@@"z""#,
                r#"/d/e/f/0@@@{}"#,
                r#"/d/e/f/1@@@9"#,
                r#"/d/e/f/2@@@"g""#,
                r#"/d/e/f/3@@@[]"#,
            ]);

            assert_eq!(challenge, expected);
        }

        /// This test exists to handle an edgecase in the RFC.
        ///
        /// Specifically:
        /// "Because the characters ~ (%x7E) and / (%x2F) have special
        /// meanings in JSON Pointer, ~ needs to be encoded as ~0 and /
        /// needs to be encoded as ~1 when these characters appear in a
        /// reference token."
        ///
        /// See:
        /// https://datatracker.ietf.org/doc/html/rfc6901#section-3
        #[test]
        fn rfc_special_chars() {
            let v: serde_json::Value = serde_json::json!(
            {
                "foo": ["bar", "baz"],
                "": 0,
                "a/b": 1,
                "c%d": 2,
                "e^f": 3,
                "g|h": 4,
                "i\\j": 5,
                "k\"l": 6,
                " ": 7,
                "m~n": 8
             }
            );

            let expected = HashSet::from([
                r#"/foo@@@["bar","baz"]"#,
                r#"/foo/0@@@"bar""#,
                r#"/foo/1@@@"baz""#,
                r#"/@@@0"#,
                r#"/a~1b@@@1"#,
                r#"/c%d@@@2"#,
                r#"/e^f@@@3"#,
                r#"/g|h@@@4"#,
                r#"/i\j@@@5"#,
                r#"/k"l@@@6"#,
                r#"/ @@@7"#,
                r#"/m~0n@@@8"#,
            ]);

            let mut challenge = Vec::new();
            let mut sink = JSONPointerWriter::new(
                &mut challenge,
                JSONPointerWriterOptions {
                    separator: "@@@",
                    only_scalars: false,
                },
            );

            jindex(&mut sink, &v).unwrap();

            let challenge = std::str::from_utf8(&challenge)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>();

            assert_eq!(challenge, expected);
        }
    }

    mod json {
        use crate::path_value_sink::{JSONWriter, JsonWriterOptions};

        use super::*;
        use std::collections::HashSet;

        #[test]
        fn simple_document() {
            let v: serde_json::Value = serde_json::json!(
                {
                    "a": 1,
                    "b": 2,
                    "c": ["x", "y", "z"],
                    "d": {"e": {"f": [{}, 9, "g"]}}
                }
            );

            let mut challenge = Vec::new();
            let mut sink = JSONWriter::new(
                &mut challenge,
                JsonWriterOptions {
                    only_scalars: false,
                },
            );

            jindex(&mut sink, &v).unwrap();

            let challenge = std::str::from_utf8(&challenge)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>();

            let expected = HashSet::from([
                r#"{"path_components":["a"],"value":1}"#,
                r#"{"path_components":["b"],"value":2}"#,
                r#"{"path_components":["c"],"value":["x","y","z"]}"#,
                r#"{"path_components":["d"],"value":{"e":{"f":[{},9,"g"]}}}"#,
                r#"{"path_components":["c",0],"value":"x"}"#,
                r#"{"path_components":["c",1],"value":"y"}"#,
                r#"{"path_components":["c",2],"value":"z"}"#,
                r#"{"path_components":["d","e"],"value":{"f":[{},9,"g"]}}"#,
                r#"{"path_components":["d","e","f"],"value":[{},9,"g"]}"#,
                r#"{"path_components":["d","e","f",0],"value":{}}"#,
                r#"{"path_components":["d","e","f",1],"value":9}"#,
                r#"{"path_components":["d","e","f",2],"value":"g"}"#,
            ]);

            assert_eq!(challenge, expected)
        }

        #[test]
        fn only_terminals() {
            let v: serde_json::Value = serde_json::json!(
                {
                    "a": 1,
                    "b": 2,
                    "c": ["x", "y", "z"],
                    "d": {"e": {"f": [{}, 9, "g", []]}}
                }
            );

            let mut challenge = Vec::new();
            let mut sink =
                JSONWriter::new(&mut challenge, JsonWriterOptions { only_scalars: true });

            jindex(&mut sink, &v).unwrap();

            let challenge = std::str::from_utf8(&challenge)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>();

            let expected = HashSet::from([
                r#"{"path_components":["a"],"value":1}"#,
                r#"{"path_components":["b"],"value":2}"#,
                r#"{"path_components":["c",0],"value":"x"}"#,
                r#"{"path_components":["c",1],"value":"y"}"#,
                r#"{"path_components":["c",2],"value":"z"}"#,
                r#"{"path_components":["d","e","f",0],"value":{}}"#,
                r#"{"path_components":["d","e","f",1],"value":9}"#,
                r#"{"path_components":["d","e","f",2],"value":"g"}"#,
                r#"{"path_components":["d","e","f",3],"value":[]}"#,
            ]);

            assert_eq!(challenge, expected);
        }
    }
}
