#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use structopt::*;

const PATH_SEPARATOR: &str = "/";

#[derive(Clone, Debug, StructOpt)]
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
    value: &'a serde_json::Value,
    path: String,
}

impl<'a> PathValue<'a> {
    fn new(value: &'a serde_json::Value, path: String) -> Self {
        Self { value, path }
    }
}

fn build_and_write_paths<W: Write>(
    writer: &mut W,
    json: Value,
    should_write_all: bool,
    separator: &str,
) -> Result<(), Box<dyn Error>> {
    let mut traversal_queue: VecDeque<PathValue> = VecDeque::new();

    let root_pathvalue = PathValue::new(&json, PATH_SEPARATOR.to_string());

    traversal_queue.push_back(root_pathvalue);

    let mut i_memo = vec![];

    while let Some(parent_pathvalue) = traversal_queue.pop_front() {
        match &parent_pathvalue.value {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    if let Some(child_pathvalue) = build_and_write_path(
                        writer,
                        k,
                        v,
                        &parent_pathvalue,
                        should_write_all,
                        separator,
                    )? {
                        traversal_queue.push_back(child_pathvalue);
                    }
                }
            }
            serde_json::Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
                    let istr = match i_memo.get(i) {
                        Some(istr) => istr,
                        None => {
                            let istr: String = i.to_string();
                            i_memo.push(istr);
                            // we call back into the vec to the the istr
                            // we just created because we must have the
                            // vec own the istr so the istr can outlive
                            // this local function
                            &i_memo[i_memo.len() - 1]
                        }
                    };

                    if let Some(child_pathvalue) = build_and_write_path(
                        writer,
                        istr,
                        v,
                        &parent_pathvalue,
                        should_write_all,
                        separator,
                    )? {
                        traversal_queue.push_back(child_pathvalue);
                    }
                }
            }
            _ => panic!("Only arrays and objects should be in the queue"),
        }
    }

    Ok(())
}

// Returns either a nonempty composite (object or array) for
// further recursion, or None if type is not a nonempty composite.
// Is a Result because `write_path` IO can fail.
fn build_and_write_path<'a, W: Write>(
    writer: &mut W,
    k: &str,
    v: &'a Value,
    parent_pathvalue: &PathValue,
    should_write_all: bool,
    separator: &str,
) -> Result<Option<PathValue<'a>>, Box<dyn Error>> {
    let child_path = build_child_path(&parent_pathvalue.path, k);

    let child_pathvalue = PathValue::new(v, child_path);

    let is_empty_composite_or_is_scalar = is_empty_composite_or_scalar(v);

    if is_empty_composite_or_is_scalar || should_write_all {
        write_path(writer, &child_pathvalue, separator)?;
    }

    if !is_empty_composite_or_is_scalar {
        Ok(Some(child_pathvalue))
    } else {
        Ok(None)
    }
}

fn build_child_path(parent_path: &str, child_path_value: &str) -> String {
    if parent_path != PATH_SEPARATOR {
        let mut child_path = String::with_capacity(parent_path.len() + 1 + child_path_value.len());
        child_path.push_str(&parent_path);
        child_path.push_str(PATH_SEPARATOR);
        child_path.push_str(child_path_value);
        child_path
    } else {
        let mut child_path = String::with_capacity(parent_path.len() + child_path_value.len());
        child_path.push_str(&parent_path);
        child_path.push_str(child_path_value);
        child_path
    }
}

fn write_path<W: Write>(
    writer: &mut W,
    pathvalue: &PathValue,
    separator: &str,
) -> Result<(), Box<dyn Error>> {
    writeln!(
        writer,
        "{}{}{}",
        pathvalue.path,
        separator,
        serde_json::to_string(&pathvalue.value)?
    )?;

    Ok(())
}

fn is_empty_composite_or_scalar(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Array(v) => v.is_empty(),
        serde_json::Value::Object(m) => m.is_empty(),
        _ => true,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();

    let v: Value = if let Some(json_location) = options.json_location {
        let mut f = File::open(json_location)?;
        let len = f.metadata()?.len();
        let mut buf = Vec::with_capacity(len.try_into()?);
        f.read_to_end(&mut buf)?;

        serde_json::from_slice(&buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let separator = &options.separator;

    let stdout = std::io::stdout();
    let mut lock = BufWriter::new(stdout.lock());

    build_and_write_paths(&mut lock, v, options.all, separator)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_simple_document() {
        let v: Value = serde_json::json!(
            {
                "a": 1,
                "b": 2,
                "c": ["x", "y", "z"],
                "d": {"e": {"f": [{}, 9, "g"]}}
            }

        );
        let mut writer = vec![];
        build_and_write_paths(&mut writer, v, true, "@@@").unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split("\n")
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>(),
            vec![
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
        let v: Value = serde_json::json!(
            {
                "a": 1,
                "b": 2,
                "c": ["x", "y", "z"],
                "d": {"e": {"f": [{}, 9, "g", []]}}
            }

        );
        let mut writer = vec![];
        build_and_write_paths(&mut writer, v, false, "@@@").unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split("\n")
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>(),
            vec![
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
