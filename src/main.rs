mod parser;
mod stats;

use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use parser::LogParser;
use stats::Stats;

/// Summarize an nginx access log in the combined format.
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// Path to the access log
    path: PathBuf,

    /// How many paths to show in the top list
    #[arg(long, default_value_t = 5)]
    top: usize,
}

enum AppError {
    Io(PathBuf, io::Error),
    Regex(regex::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Io(path, err) => write!(f, "cannot read {}: {err}", path.display()),
            AppError::Regex(err) => write!(f, "bad log pattern: {err}"),
        }
    }
}

impl From<regex::Error> for AppError {
    fn from(err: regex::Error) -> Self {
        AppError::Regex(err)
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(stats) => {
            print_report(&stats, cli.top);
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<Stats, AppError> {
    let parser = LogParser::new()?;
    let file = File::open(&cli.path).map_err(|e| AppError::Io(cli.path.clone(), e))?;

    let mut stats = Stats::default();
    // Read line by line instead of loading the whole file: access logs can be huge.
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| AppError::Io(cli.path.clone(), e))?;
        match parser.parse(&line) {
            Ok(entry) => stats.add(entry),
            Err(_) => stats.add_invalid(),
        }
    }
    Ok(stats)
}

fn print_report(stats: &Stats, top: usize) {
    println!("Total requests: {}", stats.total);
    println!("Skipped lines:  {}", stats.invalid);
    println!("Unique IPs:     {}", stats.unique_ips());
    println!("Total bytes:    {}", stats.total_bytes);

    println!();
    println!("Top {top} paths:");
    for (path, count) in stats.top_paths(top) {
        println!("  {count:>6}  {path}");
    }

    println!();
    println!("Requests by method:");
    for (method, count) in stats.methods() {
        println!("  {count:>6}  {method}");
    }

    println!();
    println!("Status classes:");
    println!("  2xx  {:>6}", stats.classes.ok);
    println!("  3xx  {:>6}", stats.classes.redirect);
    println!("  4xx  {:>6}", stats.classes.client_err);
    println!("  5xx  {:>6}", stats.classes.server_err);

    println!();
    println!("5xx by hour:");
    if stats.errors_by_hour.is_empty() {
        println!("  none");
    }
    for (hour, count) in &stats.errors_by_hour {
        println!("  {hour:02}:00  {count:>6}");
    }
}
