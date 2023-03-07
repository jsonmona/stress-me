use anyhow::{Context, Result};
use clap::Parser;
use crc::{Crc, CRC_32_CKSUM};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{atomic, Arc};

const ABOUT: &str = "Simple stress program to reach 100% CPU utilization";

/// Main program argument
#[derive(Parser, Serialize, Deserialize, Debug)]
#[command(author, version, about, long_about = ABOUT)]
struct Args {
    /// Load config from a json file
    #[arg(short, long)]
    #[serde(skip)]
    config: Option<PathBuf>,

    /// Dump current config to standard output and quit
    #[arg(long, default_value_t = false)]
    #[serde(skip)]
    save: bool,

    /// Duration to run stress for
    #[arg(short, long, default_value_t = String::from("10s"))]
    time: String,

    /// Number of threads to use
    #[arg(short, long, default_value_t = 1)]
    jobs: usize,
}

fn main() -> Result<()> {
    let mut args = Args::parse();
    if args.save {
        save_config(&args).context("writing json")?;
        return Ok(());
    }

    if let Some(x) = args.config {
        let config = std::fs::read_to_string(x)?;
        args = deser_hjson::from_str(&config)?;
    }

    let flag_quit = Arc::new(AtomicBool::new(false));
    let mut workers = Vec::with_capacity(args.jobs);
    let dur = humantime::parse_duration(&args.time)?;

    for _ in 0..args.jobs {
        let flag_quit_cloned = flag_quit.clone();
        workers.push(std::thread::spawn(move || worker_fn(flag_quit_cloned)));
    }

    std::thread::sleep(dur);
    flag_quit.store(true, atomic::Ordering::Relaxed);

    for worker in workers.into_iter() {
        worker.join().expect("worker thread panicked");
    }

    Ok(())
}

fn worker_fn(flag_quit: Arc<AtomicBool>) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_CKSUM);

    let mut rng = rand::thread_rng();
    let mut buf = [0; 256];
    let mut hash = crc.digest();

    while !flag_quit.load(atomic::Ordering::Relaxed) {
        rng.fill_bytes(&mut buf);
        hash.update(&buf);
    }

    hash.finalize()
}

fn save_config(args: &Args) -> Result<()> {
    let mut writer = std::io::stdout().lock();
    writer.write_all(b"// Config for stress-me\n")?;
    writer.write_all(b"// This config uses hjson. Please refer to https://hjson.github.io/ for more information.\n")?;
    serde_json::to_writer_pretty(writer, &args).context("writing json")
}
