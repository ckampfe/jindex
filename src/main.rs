#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::Result;
use jindex::jindex;
use jindex::path_value_sink::{GronWriter, JSONPointerWriter, JSONPointerWriterOptions};
use std::fmt::Display;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;

/// Enumerate the paths through a JSON document.
#[derive(StructOpt)]
#[structopt(name = "jindex")]
struct Options {
    /// gron, json_pointer, json
    #[structopt(short, long, default_value = "gron")]
    format: OutputFormat,
    /// A JSON file path
    #[structopt(parse(from_str))]
    json_location: Option<PathBuf>,
}

#[derive(Debug)]
enum OutputFormat {
    Gron,
    JSONPointer,
    Json,
}

#[derive(Debug)]
struct OutputFormatError(String);

impl<'a> Display for OutputFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for OutputFormat {
    type Err = OutputFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gron" => Ok(Self::Gron),
            "json_pointer" => Ok(Self::JSONPointer),
            "json" => Ok(Self::Json),
            other => Err(OutputFormatError(other.to_owned())),
        }
    }
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
        let len = f.metadata().map(|m| m.len() as usize + 1).unwrap_or(0);
        let mut buf = Vec::with_capacity(len);
        f.read_to_end(&mut buf)?;

        serde_json::from_slice(&buf)?
    } else {
        serde_json::from_reader(std::io::stdin())?
    };

    let leaked_value = ManuallyDrop::new(value);

    let stdout = std::io::stdout();

    let mut lock = BufWriter::new(stdout.lock());

    match options.format {
        OutputFormat::Gron => {
            let mut sink = GronWriter::new(&mut lock);
            jindex(&mut sink, &leaked_value)?;
        }
        OutputFormat::JSONPointer => {
            let json_pointer_writer_options = JSONPointerWriterOptions::default();
            let mut sink = JSONPointerWriter::new(&mut lock, json_pointer_writer_options);
            jindex(&mut sink, &leaked_value)?;
        }
        OutputFormat::Json => {
            return Err(anyhow::anyhow!(
                "JSON output is not yet implemented. Try `gron` or `json_pointer` instead."
            ))
        }
    }

    lock.flush()?;

    Ok(())
}
