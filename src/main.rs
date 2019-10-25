use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use structopt::*;

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "jindex")]
struct Options {
    #[structopt(parse(from_str))]
    json_location: Option<PathBuf>,
}

fn build_and_write_paths<W: Write>(json: Value, writer: &mut W) -> Result<(), Box<dyn Error>> {
    let mut q: VecDeque<(Vec<serde_json::Value>, serde_json::Value)> = VecDeque::new();

    q.push_back((vec![], json));

    while let Some((path, el)) = q.pop_front() {
        match el {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    let mut cloned_path = path.clone();
                    cloned_path.push(Value::String(k));

                    let cloned = (cloned_path, v);

                    write_path(&cloned, writer)?;
                    q.push_back(cloned)
                }
            }
            serde_json::Value::Array(a) => {
                for (i, e) in a.into_iter().enumerate() {
                    let mut cloned_path = path.clone();

                    cloned_path.push(Value::Number(
                        serde_json::Number::from_f64(i as f64).unwrap(),
                    ));

                    let cloned = (cloned_path, e);

                    write_path(&cloned, writer)?;
                    q.push_back(cloned)
                }
            }
            _ => (),
        }
    }

    Ok(())
}

fn write_path<W: Write>(
    path_value: &(Vec<serde_json::Value>, serde_json::Value),
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    let (path, value) = path_value;

    let initial_string = String::new();

    let mapped_path = path.iter().fold(initial_string, |acc, item| match item {
        serde_json::Value::String(s) => {
            if acc.is_empty() {
                format!("{}\"{}\"", acc, s.to_string())
            } else {
                format!("{}, \"{}\"", acc, s.to_string())
            }
        }
        serde_json::Value::Number(n) => {
            if acc.is_empty() {
                format!("{}{}", acc, n.as_f64().unwrap())
            } else {
                format!("{}, {}", acc, n.as_f64().unwrap())
            }
        }
        _ => panic!("JSON path items must be numbers or strings"),
    });

    let result_string = format!("[{}]", mapped_path);

    writeln!(
        writer,
        "{} => {}",
        result_string,
        serde_json::to_string(&value)?
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let options = Options::from_args();
    let mut buf = String::new();

    if let Some(json_location) = options.json_location {
        let mut f = File::open(json_location)?;
        f.read_to_string(&mut buf)?;
    } else {
        std::io::stdin().read_to_string(&mut buf)?;
    };

    let v: Value = serde_json::from_str(&buf)?;

    let mut stdout = std::io::stdout();

    build_and_write_paths(v, &mut stdout)?;

    Ok(())
}
