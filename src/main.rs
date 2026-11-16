//! Command-line interface for scanning LabVIEW folders.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

mod scan;

#[derive(Debug, Parser)]
#[command(
    name = "wirelint",
    version,
    about = "Scan folders for LabVIEW files",
    long_about = "Recursively discovers supported LabVIEW files by extension. It does not parse file contents or run analysis rules yet."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Recursively list LabVIEW file candidates in a folder.
    #[command(
        long_about = "Recursively list LabVIEW file candidates by extension. Supported extensions are .vi, .vit, .vim, .ctl, .lvproj, .lvclass, and .lvlib; matching is case-insensitive. .llb archives are not included. Symbolic links below the scan root are skipped, and file contents are not parsed."
    )]
    Scan {
        /// Folder to scan.
        #[arg(value_name = "FOLDER")]
        folder: PathBuf,
    },
}

/// Parses the command line and returns the selected command's exit status.
fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("wirelint: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Executes the selected command.
fn run(cli: Cli) -> std::io::Result<()> {
    match cli.command {
        Command::Scan { folder } => {
            let files = scan::discover_files(&folder)?;
            if files.is_empty() {
                println!("No supported LabVIEW files found.");
            } else {
                println!("Found {} supported LabVIEW file(s):", files.len());
                for file in files {
                    println!("{}", file.display());
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command};
    use clap::Parser;

    #[test]
    fn scan_subcommand_accepts_a_folder() {
        let cli = Cli::try_parse_from(["wirelint", "scan", "project"]).unwrap();

        assert!(matches!(
            cli.command,
            Command::Scan { folder } if folder.to_str() == Some("project")
        ));
    }
}
