use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::rc::Rc;
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
    parent: Option<Rc<PathNode<'a>>>,
}

impl<'a> PathNode<'a> {
    fn new(
        value: &'a serde_json::Value,
        path_value: StringOrIndex<'a>,
        parent: Option<Rc<PathNode<'a>>>,
    ) -> Self {
        Self {
            value,
            path_value,
            parent,
        }
    }

    fn compute_path(&self) -> VecDeque<&PathNode> {
        let mut path = VecDeque::new();

        path.push_front(self);

        let mut this_parent: &Option<Rc<PathNode>> = &self.parent;

        while let Some(parent) = this_parent {
            path.push_front(parent);
            this_parent = &parent.parent;
        }

        path
    }
}

fn build_and_write_paths<W: Write>(
    json: Value,
    writer: &mut W,
    write_pred: impl Fn(&serde_json::Value) -> bool,
    separator: &str,
) -> Result<(), Box<dyn Error>> {
    let mut q: VecDeque<Rc<PathNode>> = VecDeque::new();

    let root = Rc::new(PathNode::new(&json, StringOrIndex::None, None));

    q.push_back(root);

    while let Some(path_node) = q.pop_front() {
        let path_node: Rc<PathNode> = path_node;

        match &path_node.value {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    let new_pathnode =
                        PathNode::new(v, StringOrIndex::String(k), Some(path_node.clone()));

                    let is_array_or_object = v.is_object() || v.is_array();

                    let should_write = write_pred(&v);

                    if should_write {
                        write_path(&new_pathnode, writer, separator)?;
                    }

                    if is_array_or_object {
                        q.push_back(Rc::new(new_pathnode));
                    }
                }
            }
            serde_json::Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
                    let new_pathnode =
                        PathNode::new(v, StringOrIndex::Index(i), Some(path_node.clone()));

                    let is_array_or_object = v.is_object() || v.is_array();

                    let should_write = write_pred(&v);

                    if should_write {
                        write_path(&new_pathnode, writer, separator)?;
                    }

                    if is_array_or_object {
                        q.push_back(Rc::new(new_pathnode))
                    }
                }
            }
            _ => panic!("Only arrays and objects should be in the queue"),
        }
    }

    Ok(())
}

fn write_path<W: Write>(
    pathnode: &PathNode,
    writer: &mut W,
    separator: &str,
) -> Result<(), Box<dyn Error>> {
    let value = &pathnode.value;
    let path = pathnode.compute_path();

    let initial_string = String::new();

    let mapped_path = path
        .iter()
        .fold(initial_string, |mut acc, item| match &item.path_value {
            StringOrIndex::String(s) => {
                if acc.is_empty() {
                    acc.push_str("\"");
                    acc.push_str(&s);
                    acc.push_str("\"");
                    acc
                } else {
                    acc.push_str(", ");
                    acc.push_str("\"");
                    acc.push_str(&s);
                    acc.push_str("\"");
                    acc
                }
            }
            StringOrIndex::Index(n) => {
                if acc.is_empty() {
                    acc.push_str(&n.to_string());
                    acc
                } else {
                    acc.push_str(", ");
                    acc.push_str(&n.to_string());
                    acc
                }
            }
            StringOrIndex::None => acc,
        });

    writeln!(
        writer,
        "[{}]{}{}",
        mapped_path,
        separator,
        serde_json::to_string(&value)?
    )?;

    Ok(())
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

    let stdout = std::io::stdout();
    let mut lock = BufWriter::new(stdout.lock());

    if options.all {
        build_and_write_paths(v, &mut lock, |_v: &serde_json::Value| true, separator)?;
    } else {
        build_and_write_paths(
            v,
            &mut lock,
            |v: &serde_json::Value| match v {
                serde_json::Value::Array(v) => v.is_empty(),
                serde_json::Value::Object(m) => m.is_empty(),
                _ => true,
            },
            separator,
        )?;
    }

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
        build_and_write_paths(v, &mut writer, |_| true, " => ").unwrap();

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
        build_and_write_paths(
            v,
            &mut writer,
            Box::new(|v: &serde_json::Value| match v {
                serde_json::Value::Array(v) => v.is_empty(),
                serde_json::Value::Object(m) => m.is_empty(),
                _ => true,
            }),
            " => ",
        )
        .unwrap();

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
