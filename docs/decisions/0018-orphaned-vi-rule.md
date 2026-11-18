# Orphaned VI rule

- Status: Accepted
- Date: 2026-11-18

## Context

A VI can retain an owning-library link after it has been removed from the
library's member list. Such a VI is linked to a library but is no longer
managed by that library, and the inconsistency is not visible from either file
in isolation.

## Decision

Add the `orphaned-vi` rule with error severity by default. Read the VI's
`VILB` owner link from the `LIvi` linker table and compare its relative
`PTH0` path with `.lvlib` files discovered in the same scan. A VI is orphaned
when the referenced library is scanned but its `Item Type="VI"` entries do not
list that VI. Nested library items are included.

Only relative owner links and member URLs that resolve within the scan root
are compared. Absolute/token paths and libraries not discovered in the scan
are outside this check, because WireLint has no reliable portable mapping for
system paths and cannot verify members of an unscanned file. Malformed
requested linker or member metadata is an analysis error, not a passing
result.

## Alternatives and consequences

- **Check only whether the owner library exists:** this misses the defining
  inconsistency, where the link is valid but the member list is stale.
- **Report links to libraries not present in the scan:** this would conflate
  external dependencies with orphaned members and produce false positives.
- **Treat missing or malformed metadata as a pass:** this could hide orphaned
  files; requested metadata errors instead fail analysis explicitly.

The rule requires a scan-wide relationship pass after each discovered file is
parsed. It does not modify VI or library files.

## References

- [Rule architecture](0006-rule-runner-and-source-only-check.md)
- [Format research and attribution](../../THIRD_PARTY_NOTICES.md)
