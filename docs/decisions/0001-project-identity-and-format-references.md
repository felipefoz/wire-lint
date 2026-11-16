# Project identity and VI format references

- Status: Accepted
- Date: 2026-11-16

## Context

The project is an open-source LabVIEW VI static-analysis tool implemented in Rust. It needs a distinct project name and reliable starting references for understanding LabVIEW VI/RSRC binary files.

## Decisions

- Name the project **WireLint**, with the description **Static Analysis for LabVIEW**.
- License the project under the MIT License.
- Describe WireLint as an independent project with a similar purpose to NI's VI Analyzer, and display a clear non-affiliation notice in the README. This clarifies product-level provenance without implying code reuse.
- Consult these projects as starting references when researching or implementing VI/RSRC binary-format support:
  - [labview-mcp](https://github.com/Zuehlke/labview-mcp)
  - [pylabview](https://github.com/mefistotelis/pylabview)
  - [pylavi](https://github.com/marcpage/pylavi)
  - [lvpm](https://github.com/labview-package-manager/lvpm)
- Acknowledge relevant format references in `THIRD_PARTY_NOTICES.md` when implementation relies on them. If code or substantial text is copied or adapted, verify the specific source file's license and preserve its required notices.

## Consequences

- Keep the Cargo package name and project-facing description aligned with WireLint.
- Keep the MIT license text and Cargo license metadata aligned.
- Keep the NI product relationship, non-affiliation, and trademark notice visible in the README. Track any borrowed code or format references separately in `THIRD_PARTY_NOTICES.md`.
- Future VI-format work should consult these references, while treating their descriptions as research sources rather than permission to copy code.
- Add `THIRD_PARTY_NOTICES.md` when implementation uses material from these sources; do not imply code reuse when only listing them as references.
