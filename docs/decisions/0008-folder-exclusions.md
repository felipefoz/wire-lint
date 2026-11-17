# Configured folder exclusions

- Status: Accepted
- Date: 2026-11-17

## Context

Repository metadata and local LabVIEW environments can contain many files
that should not be analyzed. Users need to exclude these directory trees
before discovery visits them. Decision 0004 planned exclusions but left their
syntax and matching behavior open.

## Decisions

- Add an optional top-level YAML `exclude-folders` list of literal directory
  names. Match complete names case-sensitively on both Windows and Linux, at
  any depth below the scan root. Do not match files or exclude the root itself.
- Prune matching child directories during traversal, rather than filtering
  discovered paths afterward. Their contents do not contribute to counts,
  findings, or analysis errors.
- Reject blank names, `.` and `..`, NUL characters, and either path separator.
  Keep matching independent of host path syntax. Wildcards are not expanded.
- Put `.git` and `.lv-venv` in the configuration template. Omitted
  `exclude-folders` or an empty list means no exclusions, preserving existing
  scans and explicit-only configuration loading.
- Defer general path/glob matching, command-line exclusions, and exclusion-list
  files. No additional dependencies are needed for literal-name matching.

## Alternatives and consequences

Root-relative paths would allow more targeted exclusions, but would not skip
every nested environment with one entry. Glob matching would be more flexible
but needs a larger syntax and precedence contract.

Case-sensitive matching gives the same name-matching behavior on both
supported platforms. Users must list alternative spellings explicitly.
Folders excluded by name cannot be selectively included at another depth in
this initial version.
