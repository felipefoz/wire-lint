# No saved breakpoints rule

- Status: Accepted
- Date: 2026-11-18

## Context

Committed VIs should not contain saved block diagram breakpoints, which can
unexpectedly suspend execution when developers run them. WireLint already
extracts saved VI state from the bounded LVSR reader, so this policy can be
checked without interpreting block diagram resources or launching LabVIEW.

## Decision

Add the `no-breakpoints` rule for VI and VI-template files. Read the LVSR
breakpoints-set flag at payload offset 24 with mask `0x20000000`, using the
documented `BREAKPOINTS_SET` interpretation in pylavi. Controls and XML
containers have no applicable state and are skipped. The rule is enabled by
default with error severity so it gates the default CI threshold. The common
`enabled` and `severity` options remain available.

The rule checks the stored flag only; it does not locate or count individual
breakpoints, clear them, or modify source files. A flagged VI must be opened
in LabVIEW and its breakpoints removed before saving and committing.

## Alternatives and consequences

- **Use the debugging-enabled flag:** that flag indicates debugging capability,
  not whether breakpoints are saved. It is a separate bit.
- **Read individual block diagram records:** this is unnecessary for a
  presence policy and would expand the parser beyond its current bounded
  settings extraction.
- **Default to warning severity:** configurable, but a warning would not fail
  the default CI threshold despite the policy prohibiting committed
  breakpoints. Error severity makes the default enforce the policy.

The save-record flag interpretation is explicitly identified by pylavi as
breakpoints-set; pylabview describes the same instrument-state word but leaves
bit 29 unknown. WireLint follows pylavi's explicit semantic label and does not
infer the flag from the separate debugging-capability bit.
