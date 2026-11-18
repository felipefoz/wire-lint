//! Rule execution and findings, separate from discovery and presentation.

use crate::config::{Config, FailOn};
use crate::formats;
use crate::rules::{BUILT_INS, Severity};
use std::fs;
use std::path::{Path, PathBuf};

/// A rule violation associated with one scan-relative file path.
#[derive(Debug)]
pub struct Finding {
    pub path: PathBuf,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

/// A file that could not be analyzed; never represented as a passing check.
#[derive(Debug)]
pub struct AnalysisError {
    pub path: PathBuf,
    pub message: String,
}

/// Findings and errors accumulated in discovery/registry order.
#[derive(Debug, Default)]
pub struct Report {
    pub analyzed: usize,
    pub findings: Vec<Finding>,
    pub errors: Vec<AnalysisError>,
}

impl Report {
    /// Errors take precedence over severity-based failure (2, 1, or 0).
    pub fn exit_status(&self, fail_on: FailOn) -> u8 {
        if !self.errors.is_empty() {
            2
        } else if self
            .findings
            .iter()
            .any(|finding| fail_on.exceeded_by(finding.severity))
        {
            1
        } else {
            0
        }
    }
}

/// Reads each file once and runs every enabled rule over the extracted facts.
///
/// File errors are recorded and scanning continues. When all rules are disabled,
/// discovered files are not read and the analyzed count is zero.
pub fn analyze(root: &Path, files: &[PathBuf], config: &Config) -> Report {
    let active: Vec<_> = BUILT_INS
        .iter()
        .filter_map(|rule| {
            let options = config.rules.get(rule.id());
            if options.is_some_and(|options| !options.enabled) {
                None
            } else {
                Some((
                    *rule,
                    options
                        .and_then(|options| options.severity)
                        .unwrap_or_else(|| rule.default_severity()),
                    options,
                ))
            }
        })
        .collect();
    let mut report = Report::default();
    if active.is_empty() {
        return report;
    }
    let read_version = active
        .iter()
        .any(|(rule, _, _)| rule.id() == "saved-version");
    let read_description = active
        .iter()
        .any(|(rule, _, _)| rule.id() == "file-description");
    let read_password = active
        .iter()
        .any(|(rule, _, _)| rule.id() == "password-set");
    for path in files {
        let result = fs::read(root.join(path)).and_then(|bytes| {
            formats::read_facts(path, &bytes, read_version, read_description, read_password)
        });
        match result {
            Ok(facts) => {
                report.analyzed += 1;
                for (rule, severity, options) in &active {
                    if let Some(message) = rule.check(&facts, *options) {
                        report.findings.push(Finding {
                            path: path.clone(),
                            rule: rule.id(),
                            severity: *severity,
                            message,
                        });
                    }
                }
            }
            Err(error) => report.errors.push(AnalysisError {
                path: path.clone(),
                message: error.to_string(),
            }),
        }
    }
    report
}
