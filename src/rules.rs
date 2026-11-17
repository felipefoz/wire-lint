//! Built-in rule contract and registry, independent of file formats and output.

use crate::formats::FileFacts;
use serde::Deserialize;
use std::fmt;

/// Ordered finding severity, independent of the configured failure threshold.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

/// Stateless analysis of facts extracted once by the format layer.
///
/// Rules do not read files, print output, or decide process exit status.
pub trait Rule: Sync {
    fn id(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn default_severity(&self) -> Severity;

    /// Returns a finding message when the facts violate this rule.
    fn check(&self, facts: &FileFacts) -> Option<&'static str>;
}

/// All built-in rules; add new implementations here without changing the runner.
pub static BUILT_INS: &[&dyn Rule] = &[&SeparateCompiledCode];

/// Requires the file owner's source-only setting to be enabled.
struct SeparateCompiledCode;

impl Rule for SeparateCompiledCode {
    fn id(&self) -> &'static str {
        "separate-compiled-code"
    }

    fn description(&self) -> &'static str {
        "Require separate compiled code from source"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, facts: &FileFacts) -> Option<&'static str> {
        (!facts.separate_compiled_code)
            .then_some("Enable separate compiled code from source (source-only setting).")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_disabled_separation_only() {
        let rule = &BUILT_INS[0];
        assert!(
            rule.check(&FileFacts {
                separate_compiled_code: false
            })
            .is_some()
        );
        assert!(
            rule.check(&FileFacts {
                separate_compiled_code: true
            })
            .is_none()
        );
    }
}
