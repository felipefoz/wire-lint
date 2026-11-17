//! Strict, explicit YAML configuration for built-in rules.

use crate::rules::{BUILT_INS, Severity};
use clap::ValueEnum;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

/// Findings at or above this level cause exit status 1; analysis errors always fail.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    #[default]
    Error,
    Warning,
    None,
}

impl FailOn {
    /// Checks the finding severity without changing the reported severity.
    pub fn exceeded_by(self, severity: Severity) -> bool {
        match self {
            Self::Error => severity >= Severity::Error,
            Self::Warning => severity >= Severity::Warning,
            Self::None => false,
        }
    }
}

/// A rule's common options; new rule-specific options need a typed schema.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleConfig {
    #[serde(default = "enabled")]
    pub enabled: bool,
    pub severity: Option<Severity>,
}

/// Omitted enablement leaves a built-in rule active.
fn enabled() -> bool {
    true
}

/// Versioned configuration with all built-ins enabled unless overridden.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config {
    #[serde(default = "schema_version")]
    version: u32,
    #[serde(default)]
    pub fail_on: FailOn,
    #[serde(default, deserialize_with = "unique_rules")]
    pub rules: BTreeMap<String, RuleConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: schema_version(),
            fail_on: FailOn::default(),
            rules: BTreeMap::new(),
        }
    }
}

/// Current YAML schema version; omitted version selects version 1.
fn schema_version() -> u32 {
    1
}

/// Rejects duplicate rule IDs rather than silently accepting the last value.
fn unique_rules<'de, D>(deserializer: D) -> Result<BTreeMap<String, RuleConfig>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct UniqueRules;
    impl<'de> serde::de::Visitor<'de> for UniqueRules {
        type Value = BTreeMap<String, RuleConfig>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a mapping of unique rule identifiers to rule options")
        }

        fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
        where
            M: serde::de::MapAccess<'de>,
        {
            let mut rules = BTreeMap::new();
            while let Some((id, options)) = map.next_entry::<String, RuleConfig>()? {
                if rules.insert(id.clone(), options).is_some() {
                    return Err(serde::de::Error::custom(format!("duplicate rule '{id}'")));
                }
            }
            Ok(rules)
        }
    }
    deserializer.deserialize_map(UniqueRules)
}

impl Config {
    /// Loads only the explicitly supplied configuration; no ancestor discovery.
    ///
    /// Returns path-qualified I/O/schema errors for invalid options or rule IDs.
    pub fn load(path: &Path) -> io::Result<Self> {
        let result = fs::read_to_string(path).and_then(|text| Self::parse(&text));
        result.map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("configuration '{}': {error}", path.display()),
            )
        })
    }

    /// Deserializes and validates a complete YAML configuration.
    fn parse(text: &str) -> io::Result<Self> {
        let config: Self = serde_yaml_ng::from_str(text)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if config.version != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported configuration version; expected 1",
            ));
        }
        for id in config.rules.keys() {
            if !BUILT_INS.iter().any(|rule| rule.id() == id) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown rule '{id}'"),
                ));
            }
        }
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_configuration_covers_all_rules_with_their_defaults() {
        let config = Config::parse(include_str!("../docs/wirelint.example.yaml")).unwrap();
        assert_eq!(config.fail_on, FailOn::default());
        assert_eq!(config.rules.len(), BUILT_INS.len());
        for rule in BUILT_INS {
            let options = config.rules.get(rule.id()).unwrap_or_else(|| {
                panic!("configuration template is missing rule '{}'", rule.id())
            });
            assert!(options.enabled, "template disables rule '{}'", rule.id());
            assert_eq!(
                options.severity,
                Some(rule.default_severity()),
                "template severity differs from the default for '{}'",
                rule.id()
            );
        }
    }

    #[test]
    fn loads_defaults_and_explicit_overrides() {
        let defaults = Config::parse("{}").unwrap();
        assert!(defaults.rules.is_empty());
        assert_eq!(defaults.fail_on, FailOn::Error);
        let config = Config::parse(
            "version: 1\nfail-on: warning\nrules:\n  separate-compiled-code:\n    enabled: false\n    severity: error\n",
        ).unwrap();
        assert_eq!(config.fail_on, FailOn::Warning);
        assert!(!config.rules["separate-compiled-code"].enabled);
        assert_eq!(
            config.rules["separate-compiled-code"].severity,
            Some(Severity::Error)
        );
        assert!(
            Config::parse("rules:\n  separate-compiled-code: {}")
                .unwrap()
                .rules["separate-compiled-code"]
                .enabled
        );
    }

    #[test]
    fn rejects_unknown_duplicate_or_invalid_configuration() {
        for yaml in [
            "version: 2",
            "version: 1\nversion: 1",
            "fail_on: warning",
            "fail-on: info",
            "rules:\n  typo: {}",
            "rules:\n  separate-compiled-code:\n    enable: false",
            "rules:\n  separate-compiled-code:\n    severity: fatal",
            "rules:\n  separate-compiled-code:\n    enabled: false\n    enabled: true",
            "rules:\n  separate-compiled-code: {}\n  separate-compiled-code: {}",
            "rules: []",
            "version: [",
            "{}\n---\n{}",
        ] {
            assert!(Config::parse(yaml).is_err(), "{yaml}");
        }
    }

    #[test]
    fn failure_threshold_is_independent_of_severity() {
        for severity in [Severity::Info, Severity::Warning, Severity::Error] {
            assert_eq!(
                FailOn::Error.exceeded_by(severity),
                severity == Severity::Error
            );
            assert_eq!(
                FailOn::Warning.exceeded_by(severity),
                severity != Severity::Info
            );
            assert!(!FailOn::None.exceeded_by(severity));
        }
    }
}
