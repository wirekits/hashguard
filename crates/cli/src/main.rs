use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

use hashguard_core::baseline::{self, Baseline};
use hashguard_core::hashing;
use hashguard_core::heuristics::Level;
use hashguard_core::scan::{self, ScanOptions};

mod report;

#[derive(Parser)]
#[command(
    name = "hashguard",
    version,
    about = "Windows binary integrity checker: hashes, Authenticode signatures, baselines"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the SHA-256 of a single file
    Hash { file: PathBuf },

    /// Scan a file or folder: verify signatures and flag suspicious binaries
    Scan {
        /// File or directory to scan (e.g. C:\Windows\System32)
        path: PathBuf,
        /// Also list files that passed every check
        #[arg(long)]
        all: bool,
        /// Emit machine-readable JSON instead of text
        #[arg(long)]
        json: bool,
        /// Text file with known-bad SHA-256 hashes, one per line
        #[arg(long, value_name = "FILE")]
        blocklist: Option<PathBuf>,
    },

    /// Record SHA-256 hashes of every executable under a folder
    Baseline {
        path: PathBuf,
        /// Where to write the baseline JSON
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
    },

    /// Compare a folder against a saved baseline
    Verify {
        path: PathBuf,
        #[arg(short, long, value_name = "FILE")]
        baseline: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

fn run() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Hash { file } => {
            let digest = hashing::sha256_file(&file)
                .with_context(|| format!("cannot read {}", file.display()))?;
            println!("{digest}  {}", file.display());
            Ok(ExitCode::SUCCESS)
        }

        Command::Scan {
            path,
            all,
            json,
            blocklist,
        } => {
            let opts = ScanOptions {
                blocklist: match blocklist {
                    Some(p) => scan::load_blocklist(&p)?,
                    None => Default::default(),
                },
            };
            let (reports, errors) = scan::scan(&path, &opts)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&reports)?);
            } else {
                report::print_scan(&reports, &errors, all);
            }
            let danger = reports.iter().any(|r| r.level == Level::Danger);
            Ok(if danger {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            })
        }

        Command::Baseline { path, output } => {
            let (files, errors) = scan::hash_tree(&path)?;
            let count = files.len();
            Baseline::new(&path, files).save(&output)?;
            println!("Baseline of {count} files written to {}", output.display());
            if !errors.is_empty() {
                eprintln!("{} files could not be read and were skipped", errors.len());
            }
            Ok(ExitCode::SUCCESS)
        }

        Command::Verify {
            path,
            baseline: baseline_path,
            json,
        } => {
            let saved = Baseline::load(&baseline_path)?;
            let (current, _errors) = scan::hash_tree(&path)?;
            let diff = baseline::diff(&saved, &current);
            if json {
                println!("{}", serde_json::to_string_pretty(&diff)?);
            } else {
                report::print_diff(&diff);
            }
            Ok(if diff.is_clean() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            })
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}
