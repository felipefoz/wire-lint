//! Command-line parsing and human-readable scan presentation.

use crate::analysis;
use crate::config::{Config, FailOn};
use crate::rules::BUILT_INS;
use crate::scan;
use clap::{Parser, Subcommand};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "wirelint",
    version,
    about = "Static analysis for LabVIEW",
    long_about = "Recursively discover LabVIEW files and run enabled built-in analysis rules. Use 'wirelint rules' to list rules. Files are read without launching LabVIEW or modifying source files."
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Discover LabVIEW files and run enabled analysis rules.
    #[command(
        long_about = "Recursively analyze .vi, .vit, .vim, .ctl, .lvproj, .lvclass, and .lvlib files (case-insensitive). Skip .llb archives and symbolic links below the root. All built-in rules are enabled by default. Exit status: 0 = success, 1 = threshold reached, 2 = configuration, I/O, or analysis error. Unsupported binary formats are errors, never passing checks."
    )]
    Scan {
        /// Folder to scan.
        #[arg(value_name = "FOLDER")]
        folder: PathBuf,
        /// Explicit YAML configuration file; no automatic configuration discovery.
        #[arg(long, value_name = "FILE")]
        config: Option<PathBuf>,
        /// Failure threshold; overrides YAML (default: error). 'none' still fails on analysis errors.
        #[arg(long, value_enum)]
        fail_on: Option<FailOn>,
    },
    /// List built-in rule IDs, descriptions, and default severities.
    Rules,
}

/// Executes the command and returns the process status, propagating I/O errors.
pub fn run(cli: Cli, output: &mut impl Write, errors: &mut impl Write) -> io::Result<u8> {
    match cli.command {
        Command::Rules => {
            for rule in BUILT_INS {
                writeln!(
                    output,
                    "{} [{}]: {}",
                    rule.id(),
                    rule.default_severity(),
                    rule.description()
                )?;
            }
            Ok(0)
        }
        Command::Scan {
            folder,
            config,
            fail_on,
        } => {
            let config = config
                .as_deref()
                .map(Config::load)
                .transpose()?
                .unwrap_or_default();
            let fail_on = fail_on.unwrap_or(config.fail_on);
            let files = scan::discover_files(&folder)?;
            if files.is_empty() {
                writeln!(output, "No supported LabVIEW files found.")?;
            } else {
                writeln!(output, "Found {} supported LabVIEW file(s):", files.len())?;
                for file in &files {
                    writeln!(output, "{}", file.display())?;
                }
            }
            let report = analysis::analyze(&folder, &files, &config);
            for finding in &report.findings {
                writeln!(
                    output,
                    "{}: {} [{}]: {}",
                    finding.path.display(),
                    finding.severity,
                    finding.rule,
                    finding.message
                )?;
            }
            for error in &report.errors {
                writeln!(
                    errors,
                    "{}: analysis error: {}",
                    error.path.display(),
                    error.message
                )?;
            }
            writeln!(
                output,
                "Analyzed {} file(s); {} finding(s); {} analysis error(s).",
                report.analyzed,
                report.findings.len(),
                report.errors.len()
            )?;
            Ok(report.exit_status(fail_on))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn accepts_scan_configuration_and_threshold() {
        let cli = Cli::try_parse_from([
            "wirelint",
            "scan",
            "project",
            "--config",
            "checks.yaml",
            "--fail-on",
            "warning",
        ])
        .unwrap();
        assert!(
            matches!(cli.command, Command::Scan { folder, config: Some(config), fail_on: Some(FailOn::Warning) }
            if folder == std::path::Path::new("project") && config == std::path::Path::new("checks.yaml"))
        );
        assert!(Cli::try_parse_from(["wirelint", "scan", ".", "--fail-on", "info"]).is_err());
    }

    #[test]
    fn help_describes_rules_and_exit_behavior() {
        let mut command = Cli::command();
        let help = command
            .find_subcommand_mut("scan")
            .unwrap()
            .render_long_help()
            .to_string();
        for expected in [
            "--config",
            "--fail-on",
            "2 =",
            ".ctl",
            "default: error",
            "symbolic links",
        ] {
            assert!(help.contains(expected), "{expected}: {help}");
        }
    }
}
