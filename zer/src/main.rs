mod manifest;

use clap::{Parser, Subcommand};
use std::fs;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "zer", about = "Deterministic helpers for zereight-review")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a review-manifest from a three-dot diff
    Manifest {
        #[arg(long)]
        repo: String,
        #[arg(long)]
        base: String,
        #[arg(long, default_value = "HEAD")]
        head: String,
        #[arg(long)]
        out: Option<String>,
    },
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Manifest { repo, base, head, out } => {
            let m = manifest::build_manifest(&repo, &base, &head).map_err(|e| e.to_string())?;
            let json = serde_json::to_string_pretty(&m).map_err(|e| e.to_string())?;
            match out {
                Some(path) => {
                    fs::write(&path, json + "\n").map_err(|e| e.to_string())?;
                    println!("{path}");
                }
                None => println!("{json}"),
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("zer: {e}");
            ExitCode::FAILURE
        }
    }
}
