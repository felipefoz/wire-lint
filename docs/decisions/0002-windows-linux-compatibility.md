# Windows and Linux compatibility

- Status: Accepted
- Date: 2026-11-16

## Context

WireLint is intended to be usable by LabVIEW developers on both Windows and Linux. Platform-specific assumptions in code or tests could make the project unreliable on one of these systems.

## Decision

Treat Windows and Linux as supported platforms. Prefer cross-platform Rust APIs, avoid assumptions about shell behavior, path separators, and filesystem semantics, and isolate platform-specific behavior when it cannot be avoided.

## Consequences

- Consider both platforms when designing features and tests.
- Validate platform-sensitive changes on Windows and Linux when possible, and report which platforms were actually checked.
- Document any platform limitations or behavior differences.
