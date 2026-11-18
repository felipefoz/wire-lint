# VI debugging status rule

- Status: Accepted
- Date: 2026-11-18

## Context

Projects may require VI debugging to be disabled for delivery or enabled for
development. A configurable rule must distinguish its own enablement from
the debugging state that violates project policy.

## Decision

Add `debugging-status`, enabled by default with warning severity. Its typed
`fail-when` option accepts `enabled` (the default) or `disabled`, selecting
the stored debugging state that produces a finding. Reject the option on
other rules and reject invalid values, including explicit nulls. Existing
severity and failure thresholds continue to determine process status.

Extract bit 9 (`DebugCapable`) from the LVSR `instrState` word at payload
offset 24, following pylabview's description. Read it alongside the existing
save-record facts without extra file reads or dependencies. VI and template
resource types (`LVIN`, `sVIN`) supply this fact; controls and XML containers
have no applicable fact and are skipped by the rule. Malformed save records
remain analysis errors, never defaulted settings.

## Alternatives and consequences

- A Boolean expected-state option would work, but `fail-when: enabled` makes
  the violating state explicit without confusing it with rule enablement.
- pylavi's debuggable mask includes both bits 9 and 30. pylabview identifies
  bit 9 as `DebugCapable` and bit 30 as unknown. Use the documented bit only
  rather than equating an unknown state with Allow debugging.
- Applying VI execution policy to controls or XML container defaults would
  report settings that do not represent the VI's own execution configuration.

The rule checks the saved flag, not a live debugging session, and never
modifies input files. Default warnings are informational to CI unless the
user selects a stricter threshold or severity.

## References

- [Rule architecture](0006-rule-runner-and-source-only-check.md)
- [Format research and attribution](../../THIRD_PARTY_NOTICES.md)
- [pylabview LVSR and instrument-state definitions](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVinstrument.py)
- [pylavi save-record flags](https://github.com/marcpage/pylavi/blob/main/pylavi/resource_types.py)
