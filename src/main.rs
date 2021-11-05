#[cfg(feature = "jemalloc")]
#[global_allocator]
static ALLOC: jemalloc::Jemalloc = jemalloc::Jemalloc;

use anyhow::Result;
use jindex::jindex;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::mem::ManuallyDrop;
use std::path::PathBuf;
use structopt::StructOpt;

/// Enumerate the paths through a JSON document.
#[derive(StructOpt)]
#[structopt(name = "jindex")]
struct Options {
    /// A JSON file path
    #[structopt(parse(from_str))]
    json_location: Option<PathBuf>,
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

    jindex(&mut lock, &leaked_value)?;

    lock.flush()?;

    Ok(())
}
