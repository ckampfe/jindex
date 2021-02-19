#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::{anyhow, Result};
use std::convert::TryInto;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::PathBuf;
use structopt::StructOpt;

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
) -> Result<()> {
    let path_pool_starting_string_capacity = options.path_pool_starting_string_capacity;

    let path_pool: lifeguard::Pool<String> = lifeguard::pool()
        .with(lifeguard::StartingSize(options.path_pool_starting_size))
        .with(lifeguard::Supplier(move || {
            String::with_capacity(path_pool_starting_string_capacity)
        }))
        .build();

    let mut traversal_stack: Vec<PathValue> = vec![];

    let root_pathvalue = PathValue::new(json, path_pool.new());

    // a cache of array indexes, as strings.
    // for example, we don't need to
    // turn `0usize` into `"0"` 1000 times,
    // we do it once and store it
    let mut i_cache: Vec<Box<str>> = vec![];

    match root_pathvalue.value {
        serde_json::Value::Object(m) => {
            traverse_object(&mut traversal_stack, m, &root_pathvalue, &path_pool)
        }
        serde_json::Value::Array(a) => traverse_array(
            &mut traversal_stack,
            a,
            &root_pathvalue,
            &mut i_cache,
            &path_pool,
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
            serde_json::Value::Object(m) if !m.is_empty() => {
                traverse_object(&mut traversal_stack, m, &pathvalue, &path_pool);
                if options.all {
                    write_path(writer, &pathvalue, &options.separator)?;
                }
            }
            serde_json::Value::Array(a) if !a.is_empty() => {
                traverse_array(
                    &mut traversal_stack,
                    a,
                    &pathvalue,
                    &mut i_cache,
                    &path_pool,
                );
                if options.all {
                    write_path(writer, &pathvalue, &options.separator)?;
                }
            }
            _terminal_value => {
                write_path(writer, &pathvalue, &options.separator)?;
            }
        }
    }

    Ok(())
}

fn traverse_object<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    m: &'a serde_json::Map<String, serde_json::Value>,
    pathvalue: &PathValue,
    path_pool: &'a lifeguard::Pool<String>,
) {
    traversal_stack.extend(
        m.iter()
            .map(|(k, v)| build_path(&path_pool, &pathvalue.path, k, v)),
    )
}

fn traverse_array<'a, 'b>(
    traversal_stack: &'b mut Vec<PathValue<'a>>,
    a: &'a [serde_json::Value],
    pathvalue: &PathValue,
    i_cache: &mut Vec<Box<str>>,
    path_pool: &'a lifeguard::Pool<String>,
) {
    traversal_stack.extend(a.iter().enumerate().map(|(i, v)| {
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

        build_path(&path_pool, &pathvalue.path, istr, v)
    }))
}

fn build_path<'a>(
    path_pool: &'a lifeguard::Pool<String>,
    existing_path: &str,
    path_addition: &str,
    v: &'a serde_json::Value,
) -> PathValue<'a> {
    let mut child_path = path_pool.new();
    child_path.reserve(existing_path.len() + PATH_SEPARATOR.len() + path_addition.len());
    child_path.push_str(existing_path);
    child_path.push_str(PATH_SEPARATOR);
    child_path.push_str(path_addition);
    PathValue::new(v, child_path)
}

fn write_path<W: Write>(mut w: &mut W, pathvalue: &PathValue, separator: &str) -> Result<()> {
    w.write(&pathvalue.path.as_bytes())?;
    w.write(separator.as_bytes())?;
    serde_json::to_writer(&mut w, pathvalue.value)?;
    w.write(NEWLINE.as_bytes())?;

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
