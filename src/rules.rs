//! Built-in rule contract and registry, independent of file formats and output.

use crate::config::{DebuggingState, RuleConfig};
use crate::formats::{FileFacts, Version};
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
    fn check(&self, facts: &FileFacts, options: Option<&RuleConfig>) -> Option<String>;
}

/// All built-in rules; add new implementations here without changing the runner.
pub static BUILT_INS: &[&dyn Rule] = &[
    &SeparateCompiledCode,
    &MinimumSavedVersion,
    &FileDescription,
    &DebuggingStatus,
];

/// Default minimum release for saved LabVIEW files.
pub const DEFAULT_MINIMUM_VERSION: Version = Version {
    major: 20,
    minor: 0,
};

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

    fn check(&self, facts: &FileFacts, _options: Option<&RuleConfig>) -> Option<String> {
        (!facts.separate_compiled_code)
            .then(|| "Enable separate compiled code from source (source-only setting).".to_owned())
    }
}

/// Requires each file's saved release to meet the configured minimum.
struct MinimumSavedVersion;

impl Rule for MinimumSavedVersion {
    fn id(&self) -> &'static str {
        "minimum-saved-version"
    }

    fn description(&self) -> &'static str {
        "Require a minimum saved LabVIEW version (default 20.0)"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, facts: &FileFacts, options: Option<&RuleConfig>) -> Option<String> {
        let minimum = options
            .and_then(|options| options.minimum_version)
            .unwrap_or(DEFAULT_MINIMUM_VERSION);
        let version = facts
            .saved_version
            .expect("active version rule requires a version fact");
        (version < minimum)
            .then(|| format!("Saved LabVIEW version {version} is below the minimum {minimum}."))
    }
}

/// Requires the file owner's native description to contain non-whitespace text.
struct FileDescription;

impl Rule for FileDescription {
    fn id(&self) -> &'static str {
        "file-description"
    }

    fn description(&self) -> &'static str {
        "Require a non-empty description for each LabVIEW file"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, facts: &FileFacts, _options: Option<&RuleConfig>) -> Option<String> {
        (!facts
            .has_description
            .expect("active file-description rule requires a description fact"))
        .then(|| "Add a non-empty description to this LabVIEW file.".to_owned())
    }
}

/// Reports VIs whose stored debugging state matches the configured policy.
struct DebuggingStatus;

impl Rule for DebuggingStatus {
    fn id(&self) -> &'static str {
        "debugging-status"
    }

    fn description(&self) -> &'static str {
        "Check VI debugging status (default: flag debugging enabled)"
    }

    fn default_severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, facts: &FileFacts, options: Option<&RuleConfig>) -> Option<String> {
        let debugging_enabled = facts.debugging_enabled?;
        let fail_when = options
            .and_then(|options| options.fail_when)
            .unwrap_or_default();
        if debugging_enabled != (fail_when == DebuggingState::Enabled) {
            return None;
        }
        Some(if debugging_enabled {
            "Debugging is enabled. Disable Allow debugging in VI Properties > Execution.".to_owned()
        } else {
            "Debugging is disabled. Enable Allow debugging in VI Properties > Execution.".to_owned()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_disabled_separation_only() {
        let rule = &BUILT_INS[0];
        assert!(
            rule.check(
                &FileFacts {
                    separate_compiled_code: false,
                    has_description: None,
                    saved_version: None,
                    debugging_enabled: None,
                },
                None
            )
            .is_some()
        );
        assert!(
            rule.check(
                &FileFacts {
                    separate_compiled_code: true,
                    has_description: None,
                    saved_version: None,
                    debugging_enabled: None,
                },
                None
            )
            .is_none()
        );
    }

    #[test]
    fn compares_releases_numerically_at_the_boundary() {
        let rule = &MinimumSavedVersion;
        for (version, violation) in [
            (
                Version {
                    major: 19,
                    minor: 9,
                },
                true,
            ),
            (
                Version {
                    major: 20,
                    minor: 0,
                },
                false,
            ),
            (
                Version {
                    major: 20,
                    minor: 1,
                },
                false,
            ),
            (
                Version {
                    major: 21,
                    minor: 0,
                },
                false,
            ),
        ] {
            let facts = FileFacts {
                separate_compiled_code: true,
                has_description: None,
                saved_version: Some(version),
                debugging_enabled: None,
            };
            assert_eq!(rule.check(&facts, None).is_some(), violation);
        }
    }

    #[test]
    fn reports_missing_or_blank_file_descriptions() {
        let rule = &FileDescription;
        for (has_description, violation) in [(Some(false), true), (Some(true), false)] {
            let facts = FileFacts {
                separate_compiled_code: true,
                has_description,
                saved_version: None,
                debugging_enabled: None,
            };
            assert_eq!(rule.check(&facts, None).is_some(), violation);
        }
    }

    #[test]
    fn debugging_policy_supports_both_states_and_skips_non_vi_files() {
        for debugging_enabled in [None, Some(false), Some(true)] {
            let facts = FileFacts {
                separate_compiled_code: true,
                has_description: None,
                saved_version: None,
                debugging_enabled,
            };
            assert_eq!(
                DebuggingStatus.check(&facts, None).is_some(),
                debugging_enabled == Some(true)
            );
            for fail_when in [DebuggingState::Enabled, DebuggingState::Disabled] {
                let options = RuleConfig {
                    enabled: true,
                    severity: None,
                    minimum_version: None,
                    fail_when: Some(fail_when),
                };
                let finding = DebuggingStatus.check(&facts, Some(&options));
                assert_eq!(
                    finding.is_some(),
                    debugging_enabled == Some(fail_when == DebuggingState::Enabled)
                );
                if let Some(message) = finding {
                    assert!(message.contains(if fail_when == DebuggingState::Enabled {
                        "Debugging is enabled. Disable"
                    } else {
                        "Debugging is disabled. Enable"
                    }));
                }
            }
        }
    }
}
