# Continuous integration

- Status: Accepted
- Date: 2026-11-16

## Context

WireLint supports Windows and Linux. Changes should be checked on both platforms before they are merged.

## Decision

Use GitHub Actions on pushes and pull requests. Run formatting checks, tests, and Clippy on standard GitHub-hosted Linux and Windows runners.

## Consequences

- Keep the workflow checks aligned with the commands in `CONTRIBUTING.md`.
- Maintain coverage for both supported operating systems.
