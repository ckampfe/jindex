use serde_json::Value;
use std::boxed::Box;
use std::collections::VecDeque;
use std::error::Error;
use std::io::{Read, Write};

fn build_and_write_paths<W: Write>(json: Value, writer: &mut W) -> Result<(), Box<dyn Error>> {
    let mut q: VecDeque<(Vec<serde_json::Value>, serde_json::Value)> = VecDeque::new();

    q.push_back((vec![], json));

    while let Some((path, el)) = q.pop_front() {
        match el {
            serde_json::Value::Object(m) => {
                for (k, v) in m {
                    let mut cloned = path.clone();
                    cloned.push(Value::String(k));

                    write_path((cloned.clone(), v.clone()), writer)?;
                    q.push_back((cloned.clone(), v.clone()))
                }
            }
            serde_json::Value::Array(a) => {
                for (i, e) in a.iter().enumerate() {
                    let mut cloned = path.clone();

                    cloned.push(Value::Number(
                        serde_json::Number::from_f64(i as f64).unwrap(),
                    ));

                    write_path((cloned.clone(), e.clone()), writer)?;
                    q.push_back((cloned.clone(), e.clone()))
                }
            }
            _ => (),
        }
    }

    Ok(())
}

fn write_path<W: Write>(
    path_value: (Vec<serde_json::Value>, serde_json::Value),
    writer: &mut W,
) -> Result<(), Box<dyn Error>> {
    let (path, value) = path_value;
    let mapped_path = path
        .iter()
        .map(|item| match item {
            serde_json::Value::String(s) => s.to_string(),
            serde_json::Value::Number(n) => n.as_f64().unwrap().to_string(),
            _ => panic!("JSON path items must be numbers or strings"),
        })
        .collect::<Vec<String>>();

    writeln!(
        writer,
        "{:?} => {}",
        mapped_path,
        serde_json::to_string(&value).unwrap()
    )?;

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut buf = String::new();

    std::io::stdin().read_to_string(&mut buf)?;

    let v: Value = serde_json::from_str(&buf)?;

    let mut stdout = std::io::stdout();

    build_and_write_paths(v, &mut stdout)?;

    Ok(())
}
