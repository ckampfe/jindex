use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use structopt::*;

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

enum StringOrIndex<'a> {
    String(&'a str),
    Index(usize),
    None,
}

struct PathNode<'a> {
    value: &'a serde_json::Value,
    path_value: StringOrIndex<'a>,
    parent: Option<usize>,
}

impl<'a> PathNode<'a> {
    fn new(
        value: &'a serde_json::Value,
        path_value: StringOrIndex<'a>,
        parent: Option<usize>,
    ) -> Self {
        Self {
            value,
            path_value,
            parent,
        }
    }

    fn compute_path(&self, pathnodes: &'a Vec<PathNode>) -> VecDeque<&'a StringOrIndex> {
        let mut path = VecDeque::new();

        path.push_front(&self.path_value);

        let mut this_parent: &Option<usize> = &self.parent;

        while let Some(parent) = this_parent {
            let parent = &pathnodes[*parent];
            let path_value = &parent.path_value;
            path.push_front(&path_value);
            this_parent = &parent.parent;
        }

        path
    }
}

fn build_and_write_paths<W: Write>(
    writer: &mut W,
    json: Value,
    should_write_all: bool,
    separator: &str,
    separator_len: usize,
) -> Result<(), Box<dyn Error>> {
    let mut traversal_queue: VecDeque<PathNode> = VecDeque::new();

    let mut pathnodes: Vec<PathNode> = Vec::new();

    let root_pathnode = PathNode::new(&json, StringOrIndex::None, None);

    traversal_queue.push_back(root_pathnode);

    while let Some(parent_path_node) = traversal_queue.pop_front() {
        match &parent_path_node.value {
            serde_json::Value::Object(m) => {
                pathnodes.push(parent_path_node);

                let parent_idx = pathnodes.len() - 1;

                for (k, v) in m {
                    let child_pathnode =
                        PathNode::new(v, StringOrIndex::String(k), Some(parent_idx));

                    let is_empty_composite_or_is_scalar = is_empty_composite_or_scalar(v);

                    if is_empty_composite_or_is_scalar || should_write_all {
                        write_path(
                            writer,
                            &child_pathnode,
                            separator,
                            separator_len,
                            &pathnodes,
                        )?;
                    }

                    if !is_empty_composite_or_is_scalar {
                        traversal_queue.push_back(child_pathnode);
                    }
                }
            }
            serde_json::Value::Array(a) => {
                pathnodes.push(parent_path_node);

                let parent_idx = pathnodes.len() - 1;

                for (i, v) in a.iter().enumerate() {
                    let child_pathnode =
                        PathNode::new(v, StringOrIndex::Index(i), Some(parent_idx));

                    let is_empty_composite_or_is_scalar = is_empty_composite_or_scalar(v);

                    if is_empty_composite_or_is_scalar || should_write_all {
                        write_path(
                            writer,
                            &child_pathnode,
                            separator,
                            separator_len,
                            &pathnodes,
                        )?;
                    }

                    if !is_empty_composite_or_is_scalar {
                        traversal_queue.push_back(child_pathnode)
                    }
                }
            }
            _ => panic!("Only arrays and objects should be in the queue"),
        }
    }

    Ok(())
}

fn write_path<W: Write>(
    writer: &mut W,
    pathnode: &PathNode,
    separator: &str,
    separator_len: usize,
    pathnodes: &Vec<PathNode>,
) -> Result<(), Box<dyn Error>> {
    let value = &pathnode.value;

    let path = pathnode.compute_path(pathnodes);

    let path_len = path.len();

    let separator_bytes = separator_len * (path_len - 1);

    let mut mapped_path = String::with_capacity(path_len + separator_bytes);

    for item in path {
        match &item {
            StringOrIndex::String(s) => {
                if mapped_path.is_empty() {
                    mapped_path.push_str("\"");
                    mapped_path.push_str(&s);
                    mapped_path.push_str("\"");
                } else {
                    mapped_path.push_str(", ");
                    mapped_path.push_str("\"");
                    mapped_path.push_str(&s);
                    mapped_path.push_str("\"");
                }
            }
            StringOrIndex::Index(n) => {
                if mapped_path.is_empty() {
                    mapped_path.push_str(&n.to_string());
                } else {
                    mapped_path.push_str(", ");
                    mapped_path.push_str(&n.to_string());
                }
            }
            StringOrIndex::None => (),
        }
    }

    writeln!(
        writer,
        "[{}]{}{}",
        mapped_path,
        separator,
        serde_json::to_string(&value)?
    )?;

    Ok(())
}

#[inline(always)]
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
        let f = File::open(json_location)?;
        let buf = BufReader::new(f);
        serde_json::from_reader(buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let separator = &options.separator;

    let separator_len = separator.len();

    let stdout = std::io::stdout();
    let mut lock = BufWriter::new(stdout.lock());

    build_and_write_paths(&mut lock, v, options.all, separator, separator_len)?;

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
        build_and_write_paths(&mut writer, v, true, " => ", 4).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split("\n")
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>(),
            vec![
                r#"["a"] => 1"#,
                r#"["b"] => 2"#,
                r#"["c"] => ["x","y","z"]"#,
                r#"["d"] => {"e":{"f":[{},9,"g"]}}"#,
                r#"["c", 0] => "x""#,
                r#"["c", 1] => "y""#,
                r#"["c", 2] => "z""#,
                r#"["d", "e"] => {"f":[{},9,"g"]}"#,
                r#"["d", "e", "f"] => [{},9,"g"]"#,
                r#"["d", "e", "f", 0] => {}"#,
                r#"["d", "e", "f", 1] => 9"#,
                r#"["d", "e", "f", 2] => "g""#,
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
        build_and_write_paths(&mut writer, v, false, " => ", 4).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split("\n")
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>(),
            vec![
                r#"["a"] => 1"#,
                r#"["b"] => 2"#,
                r#"["c", 0] => "x""#,
                r#"["c", 1] => "y""#,
                r#"["c", 2] => "z""#,
                r#"["d", "e", "f", 0] => {}"#,
                r#"["d", "e", "f", 1] => 9"#,
                r#"["d", "e", "f", 2] => "g""#,
                r#"["d", "e", "f", 3] => []"#,
            ]
        )
    }
}
