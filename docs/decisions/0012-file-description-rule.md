# File description rule

- Status: Accepted
- Date: 2026-11-18

## Context

LabVIEW files expose native descriptions, but their storage differs between
VI-family binaries and XML project, class, and library documents. A rule named
for VIs alone would not represent the full supported scope.

## Decision

Add the enabled-by-default `file-description` rule with warning severity for
`.vi`, `.vit`, `.vim`, `.ctl`, `.lvclass`, `.lvlib`, and `.lvproj`. A
description is present when its owner-level native field contains
non-whitespace text. Context Help is outside the rule's scope.

Keep the format-specific readers in separate `formats::rsrc` and
`formats::xml` modules, with shared fact types and dispatch in `formats`.
Binary VI-family files use the first `STRG` string; XML documents use the
owner's `NI.Project.Description` or `NI.Lib.Description` `Str` property.
Missing or blank descriptions are findings, while malformed description
metadata is an analysis error.

## Alternatives and consequences

Calling the rule `vi-documentation` would imply it applies only to VIs and
could suggest that Context Help is checked. `file-description` communicates
the field and broad file scope. Keeping parsers in separate modules isolates
format-specific layouts while allowing analysis rules to consume common facts.
Description extraction is requested only when the rule is enabled.

[Decision 0019](0019-bookmark-rule-suppression.md) extends XML description
extraction to read suppression directives whenever rules are enabled, and
excludes directive lines from the non-empty description check.

## References

- [Project identity and VI format references](0001-project-identity-and-format-references.md)
- [Rule runner and source-only check](0006-rule-runner-and-source-only-check.md)
