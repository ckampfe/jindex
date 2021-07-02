#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::{anyhow, Result};
use smallvec::smallvec;
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

type Path = smallvec::SmallVec<[u32; 14]>;

struct PathValue<'a> {
    value: &'a serde_json::Value,
    // https://users.rust-lang.org/t/use-case-for-box-str-and-string/8295
    path: Path,
}

impl<'a> PathValue<'a> {
    fn new(value: &'a serde_json::Value, path: Path) -> Self {
        Self { value, path }
    }
}

fn build_and_write_paths<'a, 'b, W: Write>(
    writer: &mut W,
    json: &'a serde_json::Value,
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    options: &Options,
) -> Result<()> {
    let mut all_paths = ManuallyDrop::new(Vec::with_capacity(5_000));

    let root_pathvalue = PathValue::new(json, smallvec![]);

    // a cache of array indexes, as strings.
    // for example, we don't need to
    // turn `0usize` into `"0"` 1000 times,
    // we do it once and store it
    let mut i_cache: Vec<Box<str>> = vec![];

    // for the root pathvalue, we run special case traversal that does not do IO.
    // it only traverses the value and adds its results to the traversal_stack.
    match root_pathvalue.value {
        serde_json::Value::Object(object) => {
            traverse_object(traversal_stack, &mut all_paths, object, &root_pathvalue)
        }
        serde_json::Value::Array(array) => traverse_array(
            traversal_stack,
            &mut all_paths,
            array,
            &root_pathvalue,
            &mut i_cache,
        ),
        input => {
            return Err(anyhow!(
                "input value must be either a JSON array or JSON object, got: {}",
                input
            ))
        }
    }

    while let Some(pathvalue) = traversal_stack.pop() {
        match pathvalue.value {
            serde_json::Value::Object(object) if !object.is_empty() => {
                traverse_object(traversal_stack, &mut all_paths, object, &pathvalue);
                if options.all {
                    write_path_as_bytes(writer, &all_paths, &pathvalue, &options.separator)?;
                }
            }
            serde_json::Value::Array(array) if !array.is_empty() => {
                traverse_array(
                    traversal_stack,
                    &mut all_paths,
                    array,
                    &pathvalue,
                    &mut i_cache,
                );
                if options.all {
                    write_path_as_bytes(writer, &all_paths, &pathvalue, &options.separator)?;
                }
            }
            _terminal_value => {
                write_path_as_bytes(writer, &all_paths, &pathvalue, &options.separator)?;
            }
        }
    }

    Ok(())
}

fn traverse_object<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    all_paths: &mut Vec<String>,
    object: &'a serde_json::Map<String, serde_json::Value>,
    pathvalue: &PathValue,
) {
    traversal_stack.extend(
        object
            .iter()
            .map(|(k, v)| build_child_pathvalue(all_paths, &pathvalue.path, k, v)),
    )
}

fn traverse_array<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    all_paths: &mut Vec<String>,
    array: &'a [serde_json::Value],
    pathvalue: &PathValue,
    i_cache: &mut Vec<Box<str>>,
) {
    traversal_stack.extend(array.iter().enumerate().map(|(i, v)| {
        let istr = match i_cache.get(i) {
            Some(istr) => istr,
            None => {
                let istr = i.to_string().into_boxed_str();
                i_cache.push(istr);
                // we call back into the vec to the the istr
                // we just created because we must have the
                // vec own the istr so the istr can outlive
                // this local function
                &i_cache[i_cache.len() - 1]
            }
        };

        build_child_pathvalue(all_paths, &pathvalue.path, istr, v)
    }))
}

fn build_child_pathvalue<'a, 'b, T: ToString>(
    all_paths: &'b mut Vec<String>,
    existing_path: &Path,
    path_addition: T,
    value: &'a serde_json::Value,
) -> PathValue<'a> {
    all_paths.push(path_addition.to_string());
    let i = all_paths.len() - 1;
    let mut child_path = existing_path.clone();
    child_path.push(i as u32);
    PathValue::new(value, child_path)
}

fn write_path_as_bytes<W: Write>(
    writer: &mut W,
    all_paths: &[String],
    pathvalue: &PathValue,
    separator: &str,
) -> std::io::Result<()> {
    let len = pathvalue.path.len();
    let path_separator_bytes = PATH_SEPARATOR.as_bytes();
    writer.write_all(path_separator_bytes)?;
    for (i, path_i) in pathvalue.path.iter().enumerate() {
        let path_i_usize = *path_i as usize;

        assert!(all_paths.len() + 1 >= path_i_usize);

        let path = &all_paths[path_i_usize];

        writer.write_all(path.as_bytes())?;

        if i < len - 1 {
            writer.write_all(path_separator_bytes)?;
        }
    }

    // writer.write_all(pathvalue.path.as_bytes())?;
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
