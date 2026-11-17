# User documentation

- Status: Accepted
- Date: 2026-11-16

## Context

WireLint is a command-line tool with user-configurable rules, exclusions, and report formats. Users need an up-to-date reference for commands and configuration, while decision records serve a separate purpose.

## Decision

- Keep user-facing documentation in `docs/user-guide/`, separate from `docs/decisions/`.
- Keep the README concise, with the project overview and links to user documentation.
- Keep quick-start commands in the CLI guide and link to it from the README so usage examples have one maintained location.
- Add user-guide pages as the CLI takes shape: getting started, CLI reference, configuration, rules, and reports.
- Use Clap to define the CLI and derive its `--help` text from those definitions.
- When CLI options, defaults, configuration, reports, or exit behavior change, update the help text, relevant user documentation, and tests in the same change.
- Use decision records to explain why behavior was chosen; user documentation explains how to use it. Link between them where the rationale helps users.

## Consequences

- Do not duplicate the full CLI reference in the README or decision records.
- Keep examples and documented defaults aligned with the implementation.
- Create guide pages when the corresponding CLI behavior is defined, rather than documenting unsettled details.
