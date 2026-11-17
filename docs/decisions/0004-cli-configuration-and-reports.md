# CLI configuration and reports

- Status: Accepted
- Date: 2026-11-16

## Context

WireLint needs a command-line interface that scans a LabVIEW project folder, runs configured analysis rules, supports exclusions, and produces reports suitable for local use and CI.

## Decisions

- Provide a `wirelint scan <folder>` command.
- Keep CLI argument definitions and simple command dispatch in `src/main.rs` while the CLI remains small. When configuration, exclusions, and report handling make it substantially more complex, move the CLI parsing and dispatch into `src/cli.rs` and keep `main.rs` as a thin entry point.
- Initially, scan recursively for `.vi`, `.vit`, `.vim`, `.ctl`, `.lvproj`, `.lvclass`, and `.lvlib` candidates, matching extensions case-insensitively. Do not include `.llb` archives.
- Skip symbolic links encountered below the scan root and print sorted paths relative to the scan root. This initial scan discovers candidates by extension only; it does not parse files, load configuration, run rules, or generate reports.
- Use YAML configuration to enable built-in analysis rules and set their options. User-authored scripts or custom checks are out of scope initially.
- Support exclusions in the configuration, repeatable `--exclude` command-line options, and an `--exclude-from <FILE>` list.
- Provide a human-readable terminal summary by default, with GitLab Code Quality JSON and GitHub SARIF as selectable report formats.
- Give each rule a default severity of `info`, `warning`, or `error`, overridable per rule in YAML.
- Keep the failure threshold independent of reported severity. Default it to `error`; allow `warning` or `none` as alternatives. Return a nonzero exit status when findings meet or exceed the threshold.

## Open design details

- Configuration loading, schema, precedence, rule IDs, and the initial rule
  defaults are resolved in [decision 0006](0006-rule-runner-and-source-only-check.md).
  Configuration discovery remains deferred; loading is explicit.
- Literal folder exclusions in YAML are resolved in
  [decision 0008](0008-folder-exclusions.md). General path/glob exclusions,
  command-line exclusions, list-file format, and precedence remain deferred.
- Rule-specific options beyond common enablement and severity.
- Default terminal summaries, finding verbosity, and elapsed scan time are
  resolved in [decision 0009](0009-summary-verbosity-and-scan-timing.md).
- Mapping internal severities to report fields, exact report fields, and output-file behavior.
