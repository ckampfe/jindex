#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::Result;
use clap::Parser;
use jindex::jindex;
use jindex::path_value_sink::{
    GronWriter, GronWriterOptions, JSONPointerWriter, JSONPointerWriterOptions, JSONWriter,
    JsonWriterOptions,
};
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use std::str::FromStr;

/// Enumerate the paths through a JSON document.
#[derive(Parser, Debug)]
#[clap(author, version, about, name = "jindex")]
struct Options {
    /// gron, json_pointer, json
    #[clap(short, long, default_value = "gron")]
    format: OutputFormat,

    /// A JSON file path
    #[clap(parse(from_str))]
    json_location: Option<PathBuf>,
}

#[derive(Debug)]
enum OutputFormat {
    Gron,
    JSONPointer,
    Json,
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gron" => Ok(Self::Gron),
            "json_pointer" => Ok(Self::JSONPointer),
            "json" => Ok(Self::Json),
            other => Err(anyhow::anyhow!(other.to_owned())),
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

    let options = Options::parse();

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
            let gron_writer_options = GronWriterOptions::default();
            let mut sink = GronWriter::new(&mut lock, gron_writer_options);
            jindex(&mut sink, &leaked_value)?;
        }
        OutputFormat::JSONPointer => {
            let json_pointer_writer_options = JSONPointerWriterOptions::default();
            let mut sink = JSONPointerWriter::new(&mut lock, json_pointer_writer_options);
            jindex(&mut sink, &leaked_value)?;
        }
        OutputFormat::Json => {
            let json_writer_options = JsonWriterOptions::default();
            let mut sink = JSONWriter::new(&mut lock, json_writer_options);
            jindex(&mut sink, &leaked_value)?;
        }
    }

    lock.flush()?;

    Ok(())
}
