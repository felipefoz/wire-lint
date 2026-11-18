# VI password warning rule

- Status: Accepted
- Date: 2026-11-18

## Context

Password-protected VIs can prevent maintainers from inspecting or editing
source. WireLint can inspect the saved VI resource metadata without opening the
VI in LabVIEW, but must distinguish an actual password from a generic locked
state.

## Decision

Add the `password-set` rule for VI and VI-template RSRC resources (`LVIN` and
`sVIN`). It is enabled by default with warning severity. Controls and XML
containers have no applicable VI password fact and are skipped.

Read the `BDPW` password-hash resource only when this rule is enabled. Compare
its first hash with the empty-password marker used by pylavi's
`TypeBDPW.has_password`; an absent `BDPW` resource means no password is set.
Malformed or ambiguous requested `BDPW` data is an analysis error, not a
passing result. Do not retain, print, or otherwise expose hash values.

## Alternatives and consequences

- **Use the LVSR `LOCKED` flag:** pylavi describes this as locked, possibly
  with a password, so it does not prove a password is set.
- **Inspect password hashes or attempt password matching:** unnecessary for
  presence detection and would expose sensitive material.
- **Apply the rule to controls or XML containers:** these do not represent
  individual VI password state in the supported format reader.

The rule reports that a password is set and suggests removing it if the VI
must remain editable. Its warning does not fail the default error threshold.
The reader validates `BDPW` data only while the rule is active.

## References

- [Rule architecture](0006-rule-runner-and-source-only-check.md)
- [Format research and attribution](../../THIRD_PARTY_NOTICES.md)
- [pylavi `TypeBDPW` and `TypeLVSR`](https://github.com/marcpage/pylavi/blob/main/pylavi/resource_types.py)
