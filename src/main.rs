//! WireLint process entry point.

use clap::Parser;
use std::io;
use std::process::ExitCode;

mod analysis;
mod cli;
mod config;
mod formats;
mod rules;
mod scan;

/// Parses the command line and returns the selected command's exit status.
fn main() -> ExitCode {
    match cli::run(
        cli::Cli::parse(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    ) {
        Ok(status) => ExitCode::from(status),
        Err(error) => {
            eprintln!("wirelint: {error}");
            ExitCode::from(2)
        }
    }
}
