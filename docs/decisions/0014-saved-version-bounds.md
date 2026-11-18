# Saved LabVIEW version bounds

- Status: Accepted
- Date: 2026-11-18
- Supersedes: rule naming and bounds policy in [decision 0010](0010-minimum-saved-version.md)

## Context

Projects may need to reject files saved in a newer LabVIEW release as well as
older releases. The `minimum-saved-version` name no longer describes both checks.

## Decision

Rename the rule to `saved-version`, retaining default enablement and warning
severity. Accept optional, inclusive `minimum-version` and `maximum-version`
bounds using the existing quoted major.minor format. With neither bound set,
retain the default minimum of `20.0` and no maximum. A maximum-only
configuration has no implicit minimum. Equal bounds require that release.
Reject explicit nulls, invalid versions, and inverted ranges during
configuration loading, including for disabled rules.

Reject the old rule ID with migration guidance instead of accepting an alias.
Use the new ID in the registry, findings, and machine reports. Preserve the
existing metadata readers, numeric comparison, and analysis-error behavior.

## Alternatives and consequences

Separate minimum and maximum rules would duplicate configuration and findings.
Keeping the former name would obscure the upper-bound behavior; an alias would
retain two names for one rule. Always retaining the default lower bound would
make maximum-only policies below `20.0` unexpectedly invalid.

Existing configurations must rename their rule key. Report fingerprints change
because rule IDs are part of their identity. Bounds check release metadata,
not whether a file can execute in a particular LabVIEW installation.
