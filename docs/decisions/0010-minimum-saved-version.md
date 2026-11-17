# Minimum saved LabVIEW version

- Status: Accepted
- Date: 2026-11-17

## Context

Projects need a check that source files were saved in a sufficiently recent
LabVIEW release. VI-family files and XML containers store their own versions;
container settings cannot establish the version of their members.

## Decision

Add the enabled-by-default `minimum-saved-version` rule with warning severity
and a typed `minimum-version` YAML option defaulting to `20.0`. Compare numeric
major and minor release components; ignore patch, build, and prerelease stage.
Read the LVSR version word in binary files and the hexadecimal root
`LVVersion` word in project, library, and class XML. Reject missing or malformed
version metadata when the rule is enabled, and do not require it when disabled.
Report the actual and required versions with each finding.

## Alternatives and consequences

A fixed threshold would require a code change for each project policy. Inferring
member versions from their container would produce inaccurate results. Parsing
version metadata only when the rule is active preserves other checks on XML
files that have no usable version. The configured minimum is not a maximum
compatibility guarantee: newer files may not open in an older LabVIEW.
