use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::rc::Rc;
use structopt::*;

#[derive(Clone, Debug, StructOpt)]
#[structopt(name = "jindex")]
struct Options {
    #[structopt(parse(from_str))]
    json_location: Option<PathBuf>,
}

fn build_and_write_paths<W: Write>(json: Value, writer: &mut W) -> Result<(), Box<dyn Error>> {
    let mut q: VecDeque<(Vec<Rc<serde_json::Value>>, serde_json::Value)> = VecDeque::new();

    q.push_back((vec![], json));

    while let Some((path, el)) = q.pop_front() {
        match el {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    let mut cloned_path = path.clone();
                    cloned_path.push(Rc::new(Value::String(k)));

                    let path_value = (cloned_path, v);

                    write_path(&path_value, writer)?;
                    q.push_back(path_value)
                }
            }
            serde_json::Value::Array(a) => {
                for (i, v) in a.into_iter().enumerate() {
                    let mut cloned_path = path.clone();

                    cloned_path.push(Rc::new(Value::Number(
                        serde_json::Number::from_f64(i as f64).unwrap(),
                    )));

                    let path_value = (cloned_path, v);

                    write_path(&path_value, writer)?;
                    q.push_back(path_value)
                }
            }
            _ => (),
        }
    }

    Ok(())
}

fn write_path<W: Write>(
    path_value: &(Vec<Rc<serde_json::Value>>, serde_json::Value),
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    let (path, value) = path_value;

    let initial_string = String::new();

    let mapped_path = path.iter().fold(initial_string, |acc, item| match &**item {
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

    let v: Value = if let Some(json_location) = options.json_location {
        let f = File::open(json_location)?;
        let buf = BufReader::new(f);
        serde_json::from_reader(buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let mut stdout = std::io::stdout();

    build_and_write_paths(v, &mut stdout)?;

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
        build_and_write_paths(v, &mut writer).unwrap();

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
}
