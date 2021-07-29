#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::Result;
use std::convert::TryInto;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use structopt::StructOpt;

const NEWLINE: &str = "\n";
const PATH_SEPARATOR: &str = "/";

/// Enumerate the paths through a JSON document.
#[derive(StructOpt)]
#[structopt(name = "jindex")]
struct Options {
    /// Write all path values, including composite ones
    #[structopt(short, long)]
    all: bool,

    /// A JSON file path
    #[structopt(parse(from_str))]
    json_location: Option<PathBuf>,

    /// Separator string, defaults to tab
    #[structopt(default_value = "\t", short, long)]
    separator: String,
}

struct PathValue<'a> {
    // https://users.rust-lang.org/t/use-case-for-box-str-and-string/8295
    path: Box<str>,
    value: &'a serde_json::Value,
}

impl<'a> PathValue<'a> {
    fn new(value: &'a serde_json::Value, path: Box<str>) -> Self {
        Self { path, value }
    }
}

struct AllIter<'a, 'b> {
    traversal_stack: &'b mut Vec<PathValue<'a>>,
}

impl<'a, 'b> AllIter<'a, 'b> {
    fn new(pathvalue: PathValue<'a>, traversal_stack: &'b mut Vec<PathValue<'a>>) -> Self {
        traversal_stack.push(pathvalue);
        let mut s = Self { traversal_stack };
        let _ = s.next();
        s
    }
}

impl<'a, 'b> Iterator for AllIter<'a, 'b> {
    type Item = PathValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pathvalue) = self.traversal_stack.pop() {
            match pathvalue.value {
                serde_json::Value::Object(object) if !object.is_empty() => {
                    traverse_object(&mut self.traversal_stack, object, &pathvalue);
                    Some(pathvalue)
                }
                serde_json::Value::Array(array) if !array.is_empty() => {
                    traverse_array(&mut self.traversal_stack, array, &pathvalue);
                    Some(pathvalue)
                }
                _value => Some(pathvalue),
            }
        } else {
            None
        }
    }
}

struct TerminalsOnlyIter<'a> {
    traversal_stack: Vec<PathValue<'a>>,
}

impl<'a> TerminalsOnlyIter<'a> {
    fn new(pathvalue: PathValue<'a>) -> Self {
        let traversal_stack = vec![pathvalue];
        Self { traversal_stack }
    }
}

impl<'a> Iterator for TerminalsOnlyIter<'a> {
    type Item = PathValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pathvalue) = self.traversal_stack.pop() {
            match pathvalue.value {
                serde_json::Value::String(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::Bool(_)
                | serde_json::Value::Null => Some(pathvalue),
                serde_json::Value::Object(object) if object.is_empty() => Some(pathvalue),
                serde_json::Value::Array(array) if array.is_empty() => Some(pathvalue),
                serde_json::Value::Object(object) => {
                    traverse_object(&mut self.traversal_stack, object, &pathvalue);
                    self.next()
                }
                serde_json::Value::Array(array) => {
                    traverse_array(&mut self.traversal_stack, array, &pathvalue);
                    self.next()
                }
            }
        } else {
            None
        }
    }
}

fn build_and_write_paths<'a, 'b, W: Write>(
    writer: &mut W,
    json: &'a serde_json::Value,
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    options: &Options,
) -> Result<()> {
    let root_pathvalue = PathValue::new(json, String::new().into_boxed_str());

    if options.all {
        let iter = AllIter::new(root_pathvalue, traversal_stack);
        for pathvalue in iter {
            write_path_as_bytes(writer, &pathvalue, &options.separator)?;
        }
    } else {
        let iter = TerminalsOnlyIter::new(root_pathvalue);
        for pathvalue in iter {
            write_path_as_bytes(writer, &pathvalue, &options.separator)?;
        }
    };

    Ok(())
}

fn traverse_object<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    object: &'a serde_json::Map<String, serde_json::Value>,
    pathvalue: &PathValue,
) {
    traversal_stack.extend(
        object
            .iter()
            .map(|(k, v)| build_child_pathvalue(&pathvalue.path, k, v)),
    )
}

fn traverse_array<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    array: &'a [serde_json::Value],
    pathvalue: &PathValue,
) {
    traversal_stack.extend(
        array
            .iter()
            .enumerate()
            .map(|(i, v)| build_child_pathvalue(&pathvalue.path, i, v)),
    )
}

fn build_child_pathvalue<'a, T: ToString>(
    existing_path: &str,
    path_addition: T,
    value: &'a serde_json::Value,
) -> PathValue<'a> {
    let path_addition = path_addition.to_string();
    let mut child_path =
        String::with_capacity(existing_path.len() + PATH_SEPARATOR.len() + path_addition.len());
    child_path.push_str(existing_path);
    child_path.push_str(PATH_SEPARATOR);
    child_path.push_str(&path_addition);
    PathValue::new(value, child_path.into_boxed_str())
}

fn write_path_as_bytes<W: Write>(
    writer: &mut W,
    pathvalue: &PathValue,
    separator: &str,
) -> std::io::Result<()> {
    writer.write_all(pathvalue.path.as_bytes())?;
    writer.write_all(separator.as_bytes())?;
    serde_json::to_writer(&mut *writer, pathvalue.value)?;
    writer.write_all(NEWLINE.as_bytes())?;
    Ok(())
}

fn main() -> Result<()> {
    // https://github.com/rust-lang/rust/issues/46016
    #[cfg(target_family = "unix")]
    {
        use nix::sys::signal;
        let _ = unsafe { signal::signal(signal::Signal::SIGPIPE, signal::SigHandler::SigDfl)? };
    }

    let options = Options::from_args();

    let value: serde_json::Value = if let Some(json_location) = &options.json_location {
        let mut f = File::open(json_location)?;
        let len = f.metadata()?.len();
        let mut buf = Vec::with_capacity(len.try_into()?);
        f.read_to_end(&mut buf)?;

        serde_json::from_slice(&buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let leaked_value = ManuallyDrop::new(value);

    let mut traversal_stack: ManuallyDrop<Vec<PathValue>> = ManuallyDrop::new(vec![]);

    let stdout = std::io::stdout();
    let mut lock = BufWriter::new(stdout.lock());

    build_and_write_paths(&mut lock, &leaked_value, &mut traversal_stack, &options)?;

    lock.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    macro_rules! hashset {
        () => {
            HashSet::new()
        };
        ( $($x:expr),+ $(,)? ) => {
            {
                let mut set = HashSet::new();

                $(
                    set.insert($x);
                )*

                set
            }
        };
    }

    #[test]
    fn a_simple_document() {
        let v: serde_json::Value = serde_json::json!(
            {
                "a": 1,
                "b": 2,
                "c": ["x", "y", "z"],
                "d": {"e": {"f": [{}, 9, "g"]}}
            }
        );
        let mut writer = vec![];

        let options = Options {
            all: true,
            json_location: None,
            separator: "@@@".to_string(),
        };

        let mut traversal_stack: Vec<PathValue> = vec![];

        build_and_write_paths(&mut writer, &v, &mut traversal_stack, &options).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>(),
            hashset![
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
            ]
        )
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
        let mut writer = vec![];

        let options = Options {
            all: false,
            json_location: None,
            separator: "@@@".to_string(),
        };

        let mut traversal_stack: Vec<PathValue> = vec![];

        build_and_write_paths(&mut writer, &v, &mut traversal_stack, &options).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split('\n')
                .filter(|s| !s.is_empty())
                .collect::<HashSet<&str>>(),
            hashset![
                r#"/a@@@1"#,
                r#"/b@@@2"#,
                r#"/c/0@@@"x""#,
                r#"/c/1@@@"y""#,
                r#"/c/2@@@"z""#,
                r#"/d/e/f/0@@@{}"#,
                r#"/d/e/f/1@@@9"#,
                r#"/d/e/f/2@@@"g""#,
                r#"/d/e/f/3@@@[]"#,
            ]
        )
    }
}
