# CLI configuration and reports

- Status: Accepted
- Date: 2026-11-16

## Context

WireLint needs a command-line interface that scans a LabVIEW project folder, runs configured analysis rules, supports exclusions, and produces reports suitable for local use and CI.

## Decisions

- Provide a `wirelint scan <folder>` command.
- Use YAML configuration to enable built-in analysis rules and set their options. User-authored scripts or custom checks are out of scope initially.
- Support exclusions in the configuration, repeatable `--exclude` command-line options, and an `--exclude-from <FILE>` list.
- Provide a human-readable terminal summary by default, with GitLab Code Quality JSON and GitHub SARIF as selectable report formats.
- Give each rule a default severity of `info`, `warning`, or `error`, overridable per rule in YAML.
- Keep the failure threshold independent of reported severity. Default it to `error`; allow `warning` or `none` as alternatives. Return a nonzero exit status when findings meet or exceed the threshold.

## Open design details

- Configuration filename discovery, schema, and precedence between file and command-line settings.
- Exclusion pattern syntax, path base, list-file format, and precedence.
- Supported LabVIEW file types and scan traversal behavior.
- Rule identifiers, default rule set, and per-rule configuration.
- Mapping internal severities to report fields, exact report fields, and output-file behavior.
