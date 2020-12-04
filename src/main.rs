#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use lifeguard::*;
use std::boxed::Box;
use std::convert::TryInto;
use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use structopt::*;

const PATH_SEPARATOR: &str = "/";
const NEWLINE: &str = "\n";

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

    /// Initial number of preallocated strings in the path pool
    #[structopt(default_value = "128", long)]
    path_pool_starting_size: usize,

    /// Preallocated capacity of the strings in the path pool
    #[structopt(default_value = "50", long)]
    path_pool_starting_string_capacity: usize,
}

struct PathValue<'a> {
    value: &'a serde_json::Value,
    // https://users.rust-lang.org/t/use-case-for-box-str-and-string/8295
    path: lifeguard::Recycled<'a, std::string::String>,
}

impl<'a> PathValue<'a> {
    fn new(
        value: &'a serde_json::Value,
        path: lifeguard::Recycled<'a, std::string::String>,
    ) -> Self {
        Self { value, path }
    }
}

fn build_and_write_paths<W: Write>(
    writer: &mut W,
    json: &serde_json::Value,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    let path_pool_starting_string_capacity = options.path_pool_starting_string_capacity;

    let path_pool: Pool<String> = pool()
        .with(StartingSize(options.path_pool_starting_size))
        .with(Supplier(move || {
            String::with_capacity(path_pool_starting_string_capacity)
        }))
        .build();

    let mut traversal_stack: Vec<PathValue> = Vec::new();

    let root_pathvalue = PathValue::new(&json, path_pool.new());

    traversal_stack.push(root_pathvalue);

    // a cache of array indexes, as strings.
    // for example, we don't need to
    // turn `0usize` into `"0"` 1000 times,
    // we do it once and store it
    let mut i_cache = vec![];

    // we buffer io writes into this,
    // flushing it only once per array or object,
    // rather than doing a write for every single path
    let mut io_buf = vec![];

    while let Some(parent_pathvalue) = traversal_stack.pop() {
        match &parent_pathvalue.value {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    build_and_write_path(
                        &path_pool,
                        &mut io_buf,
                        &mut traversal_stack,
                        k,
                        v,
                        &parent_pathvalue,
                        options.all,
                        &options.separator,
                    )?;
                }
            }
            serde_json::Value::Array(a) => {
                for (i, v) in a.iter().enumerate() {
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

                    build_and_write_path(
                        &path_pool,
                        &mut io_buf,
                        &mut traversal_stack,
                        istr,
                        v,
                        &parent_pathvalue,
                        options.all,
                        &options.separator,
                    )?;
                }
            }
            _ => panic!("Only arrays and objects should be in the stack"),
        }

        writer.write_all(&io_buf)?;
        io_buf.clear();
    }

    Ok(())
}

// Returns either a nonempty composite (object or array) for
// further recursion, or None if type is not a nonempty composite.
// Is a Result because `write_path` IO can fail.
#[allow(clippy::too_many_arguments)]
fn build_and_write_path<'a>(
    path_pool: &'a lifeguard::Pool<String>,
    io_buf: &mut Vec<u8>,
    traversal_stack: &mut Vec<PathValue<'a>>,
    k: &str,
    v: &'a serde_json::Value,
    parent_pathvalue: &PathValue,
    should_write_all: bool,
    separator: &str,
) -> serde_json::Result<()> {
    let child_path = build_child_path(&path_pool, &parent_pathvalue.path, k);

    let child_pathvalue = PathValue::new(v, child_path);

    if is_terminal(v) {
        write_path(io_buf, &child_pathvalue, separator)?;
    } else if should_write_all {
        write_path(io_buf, &child_pathvalue, separator)?;
        traversal_stack.push(child_pathvalue);
    } else {
        traversal_stack.push(child_pathvalue);
    }

    Ok(())
}

fn build_child_path<'a>(
    path_pool: &'a lifeguard::Pool<String>,
    parent_path: &str,
    child_path_value: &str,
) -> lifeguard::Recycled<'a, std::string::String> {
    let mut child_path = path_pool.new();
    child_path.reserve(parent_path.len() + PATH_SEPARATOR.len() + child_path_value.len());
    child_path.push_str(parent_path);
    child_path.push_str(PATH_SEPARATOR);
    child_path.push_str(child_path_value);
    child_path
}

fn write_path(
    mut io_buf: &mut Vec<u8>,
    pathvalue: &PathValue,
    separator: &str,
) -> serde_json::Result<()> {
    io_buf.extend_from_slice(&pathvalue.path.as_bytes());
    io_buf.extend_from_slice(separator.as_bytes());
    serde_json::to_writer(&mut io_buf, pathvalue.value)?;
    io_buf.extend_from_slice(NEWLINE.as_bytes());

    Ok(())
}

// a terminal is an empty object, an empty array,
// or any other value
fn is_terminal(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::Array(v) => v.is_empty(),
        serde_json::Value::Object(m) => m.is_empty(),
        _ => true,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // https://github.com/rust-lang/rust/issues/46016
    #[cfg(target_family = "unix")]
    {
        use nix::sys::signal;
        let _ = unsafe { signal::signal(signal::Signal::SIGPIPE, signal::SigHandler::SigDfl)? };
    }

    let options = Options::from_args();

    let v: serde_json::Value = if let Some(json_location) = &options.json_location {
        let mut f = File::open(json_location)?;
        let len = f.metadata()?.len();
        let mut buf = Vec::with_capacity(len.try_into()?);
        f.read_to_end(&mut buf)?;

        serde_json::from_slice(&buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let stdout = std::io::stdout();
    let mut lock = BufWriter::new(stdout.lock());

    build_and_write_paths(&mut lock, &v, &options)?;

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
            path_pool_starting_size: 128,
            path_pool_starting_string_capacity: 50,
        };

        build_and_write_paths(&mut writer, &v, &options).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split(NEWLINE)
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
            path_pool_starting_size: 128,
            path_pool_starting_string_capacity: 50,
        };

        build_and_write_paths(&mut writer, &v, &options).unwrap();

        assert_eq!(
            std::str::from_utf8(&writer)
                .unwrap()
                .split(NEWLINE)
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
