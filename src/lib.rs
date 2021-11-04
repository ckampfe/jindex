use anyhow::{anyhow, Result};
use serde_json::json;
use std::io::Write;

struct PathValue<'a> {
    value: &'a serde_json::Value,
    path_components: Vec<KeyOrIndex<'a>>,
}

impl<'a> PathValue<'a> {
    fn new(value: &'a serde_json::Value, path_components: Vec<KeyOrIndex<'a>>) -> Self {
        Self {
            value,
            path_components,
        }
    }
}

// on apple aarch64 size_of reports this as being of size 24
#[derive(Clone, Debug)]
enum KeyOrIndex<'a> {
    Identifier(&'a str),
    NonIdentifier(&'a str),
    Index(usize),
}

// necessary for use with TinyVec
impl Default for KeyOrIndex<'_> {
    fn default() -> Self {
        KeyOrIndex::Index(0)
    }
}

/// Enumerate the paths through a JSON document.
pub fn jindex<W: Write>(writer: &mut W, json: &serde_json::Value) -> Result<()> {
    let mut traversal_stack: Vec<PathValue> = vec![];

    let root_pathvalue = PathValue::new(json, Vec::new());
    let mut length_total = 0;
    let mut paths_seen = 0;
    let mut average_path_length;

    // for the root pathvalue, we run special case traversal that does not do IO.
    // it only traverses the value and adds its results to the traversal_stack.
    match root_pathvalue.value {
        serde_json::Value::Object(object) => {
            write_path_as_bytes(writer, &PathValue::new(&json!({}), Vec::new()))?;
            traverse_object(&mut traversal_stack, object, &root_pathvalue, 0);
        }
        serde_json::Value::Array(array) => {
            write_path_as_bytes(writer, &PathValue::new(&json!([]), Vec::new()))?;
            traverse_array(&mut traversal_stack, array, &root_pathvalue, 0)
        }
        input => {
            return Err(anyhow!(
                "input value must be either a JSON array or JSON object, got: {}",
                input
            ))
        }
    }

    while let Some(pathvalue) = traversal_stack.pop() {
        length_total += pathvalue.path_components.len();
        paths_seen += 1;

        average_path_length = length_total / paths_seen + 1;

        match pathvalue.value {
            serde_json::Value::Object(object) => {
                traverse_object(
                    &mut traversal_stack,
                    object,
                    &pathvalue,
                    average_path_length,
                );
            }
            serde_json::Value::Array(array) => {
                traverse_array(&mut traversal_stack, array, &pathvalue, average_path_length);
            }
            _terminal_value => (),
        }

        write_path_as_bytes(writer, &pathvalue)?;
    }

    Ok(())
}

fn traverse_object<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    object: &'a serde_json::Map<String, serde_json::Value>,
    pathvalue: &PathValue<'a>,
    path_allocation_size: usize,
) {
    traversal_stack.extend(object.iter().map(|(k, v)| {
        let mut cloned = Vec::with_capacity(path_allocation_size);

        cloned.clone_from(&pathvalue.path_components);

        let component = if is_identifier(k) {
            KeyOrIndex::Identifier(k)
        } else {
            KeyOrIndex::NonIdentifier(k)
        };

        cloned.push(component);

        PathValue::new(v, cloned)
    }))
}

fn traverse_array<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    array: &'a [serde_json::Value],
    pathvalue: &PathValue<'a>,
    path_allocation_size: usize,
) {
    traversal_stack.extend(array.iter().enumerate().map(|(i, v)| {
        let mut cloned = Vec::with_capacity(path_allocation_size);

        cloned.clone_from(&pathvalue.path_components);

        cloned.push(KeyOrIndex::Index(i));

        PathValue::new(v, cloned)
    }))
}

fn write_path_as_bytes<W: Write>(writer: &mut W, pathvalue: &PathValue) -> std::io::Result<()> {
    writer.write_all(b"json")?;

    for path_component in &pathvalue.path_components {
        match path_component {
            KeyOrIndex::Identifier(s) => {
                writer.write_all(b".")?;
                writer.write_all(s.as_bytes())?;
            }
            KeyOrIndex::NonIdentifier(s) => {
                writer.write_all(b"[\"")?;
                writer.write_all(s.as_bytes())?;
                writer.write_all(b"\"]")?;
            }
            KeyOrIndex::Index(i) => {
                writer.write_all(b"[")?;
                itoa::write(&mut *writer, *i)?;
                writer.write_all(b"]")?;
            }
        }
    }

    writer.write_all(b" = ")?;

    match pathvalue.value {
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Null
        | serde_json::Value::Bool(_) => {
            serde_json::to_writer(&mut *writer, pathvalue.value)?;
        }
        serde_json::Value::Array(_) => writer.write_all(b"[]")?,
        serde_json::Value::Object(_) => writer.write_all(b"{}")?,
    }

    writer.write_all(b";\n")?;

    Ok(())
}

// TODO make this real?
// TODO make this fast?
fn is_identifier(s: &str) -> bool {
    if s.starts_with('0')
        || s.starts_with('1')
        || s.starts_with('1')
        || s.starts_with('2')
        || s.starts_with('3')
        || s.starts_with('4')
        || s.starts_with('5')
        || s.starts_with('6')
        || s.starts_with('7')
        || s.starts_with('8')
        || s.starts_with('9')
        || s.contains('-')
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn gron_one() {
        let mut expected = String::new();
        std::fs::File::open("fixtures/one.gron")
            .unwrap()
            .read_to_string(&mut expected)
            .unwrap();

        let mut expected: Vec<&str> = expected.split("\n").collect();
        expected.sort();

        let mut input = String::new();
        std::fs::File::open("fixtures/one.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        jindex(&mut challenge, &parsed).unwrap();
        let challenge = String::from_utf8(challenge).unwrap();
        let challenge = challenge.trim();

        let mut challenge: Vec<&str> = challenge.split("\n").collect();
        challenge.sort();

        assert_eq!(expected, challenge);
    }

    #[test]
    fn gron_two() {
        let mut expected = String::new();
        std::fs::File::open("fixtures/two.gron")
            .unwrap()
            .read_to_string(&mut expected)
            .unwrap();

        let mut expected: Vec<&str> = expected.split("\n").collect();
        expected.sort();

        let mut input = String::new();
        std::fs::File::open("fixtures/two.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        jindex(&mut challenge, &parsed).unwrap();
        let challenge = String::from_utf8(challenge).unwrap();
        let challenge = challenge.trim();

        let mut challenge: Vec<&str> = challenge.split("\n").collect();
        challenge.sort();

        assert_eq!(expected, challenge);
    }

    #[test]
    fn gron_three() {
        let mut expected = String::new();
        std::fs::File::open("fixtures/three.gron")
            .unwrap()
            .read_to_string(&mut expected)
            .unwrap();

        let mut expected: Vec<&str> = expected.split("\n").collect();
        expected.sort();

        let mut input = String::new();
        std::fs::File::open("fixtures/three.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        jindex(&mut challenge, &parsed).unwrap();
        let challenge = String::from_utf8(challenge).unwrap();
        let challenge = challenge.trim();

        let mut challenge: Vec<&str> = challenge.split("\n").collect();
        challenge.sort();

        assert_eq!(expected, challenge);
    }

    #[test]
    fn gron_github() {
        let mut expected = String::new();
        std::fs::File::open("fixtures/github.gron")
            .unwrap()
            .read_to_string(&mut expected)
            .unwrap();

        let mut expected: Vec<&str> = expected.split("\n").collect();
        expected.sort();

        let mut input = String::new();
        std::fs::File::open("fixtures/github.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        jindex(&mut challenge, &parsed).unwrap();
        let challenge = String::from_utf8(challenge).unwrap();
        let challenge = challenge.trim();

        let mut challenge: Vec<&str> = challenge.split("\n").collect();
        challenge.sort();

        assert_eq!(expected, challenge);
    }

    #[test]
    fn gron_large_line() {
        let mut expected = String::new();
        std::fs::File::open("fixtures/large-line.gron")
            .unwrap()
            .read_to_string(&mut expected)
            .unwrap();

        let mut expected: Vec<&str> = expected.split("\n").collect();
        expected.sort();

        let mut input = String::new();
        std::fs::File::open("fixtures/large-line.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        jindex(&mut challenge, &parsed).unwrap();
        let challenge = String::from_utf8(challenge).unwrap();
        let challenge = challenge.trim();

        let mut challenge: Vec<&str> = challenge.split("\n").collect();
        challenge.sort();

        assert_eq!(expected, challenge);
    }

    #[test]
    fn gron_big() {
        // 923k is not really that big but this is what gron itself uses
        let mut input = String::new();
        std::fs::File::open("fixtures/large-line.json")
            .unwrap()
            .read_to_string(&mut input)
            .unwrap();

        let parsed = serde_json::from_str(&input).unwrap();

        let mut challenge = Vec::new();
        // simply asserting that we don't panic here
        jindex(&mut challenge, &parsed).unwrap();

        assert!(true)
    }
}
