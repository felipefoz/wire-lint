# Analysis rules

WireLint calls its static-analysis tests **rules**, to distinguish them from
the Rust tests that verify WireLint itself. List the built-in rules with:

```text
wirelint rules
```

All built-ins are enabled by default. Each has a stable ID, default severity,
and shared `enabled` and `severity` configuration options. See
[configuration](configuration.md) for YAML examples and exit thresholds.

## Execution order

WireLint processes discovered files sequentially, in sorted path order. For
each file, it reads and parses the contents once into shared facts, then runs
every enabled rule sequentially in built-in registry order before moving to
the next file:

```text
File A: read/parse once -> Rule 1 -> Rule 2 -> ...
File B: read/parse once -> Rule 1 -> Rule 2 -> ...
```

Rules reuse the shared facts without rereading the file. Neither files nor
rules run in parallel. A finding does not stop the remaining rules or files.
If reading or parsing fails, WireLint records an analysis error, skips the
rules for that file, and continues with the next file.

If all rules are disabled, discovery still runs, but file contents are not
read or parsed. Findings and errors are reported after analysis finishes.

## `separate-compiled-code`

Requires **Separate compiled code from source** to be enabled.

| Setting | Value |
| --- | --- |
| Default enablement | Enabled |
| Default severity | `warning` |
| Applies to | `.vi`, `.vit`, `.vim`, `.ctl`, `.lvlib`, `.lvclass`, `.lvproj` |
| Rule-specific options | None |

Keeping compiled code separate helps avoid source-control changes caused only
by recompilation. This rule checks the stored setting, not the presence or
absence of a compiled-code resource.

### How the setting is read

- **Binary VI-family files** (`.vi`, `.vit`, `.vim`, `.ctl`): locate the `LVSR`
  save-record resource through the RSRC metadata. Read bit 10 (`0x00000400`,
  `SourceOnly`) in the big-endian `viFlags2` word at offset 8 of the LVSR
  payload. A clear bit means false.
- **XML files**: read the direct child property named `NI.LV.All.SourceOnly`,
  with `Type="Bool"`, on the document root (`Project`, `Library`, or `LVClass`).
  `true` passes; `false` or an absent property produces a finding. Whitespace
  around Boolean text is ignored. Nested items and comments do not supply the
  owner's setting.

Each discovered file is checked independently. Project, library, and class
properties control separation for new items; enabling them does not prove that
existing members have separation enabled. WireLint does not inherit these
values, follow member references outside the scanned folder, or modify files.

### Configuration

```yaml
rules:
  separate-compiled-code:
    enabled: true
    severity: warning
```

The expected value is always true. Disable the rule to opt out; there is no
option to require embedded compiled code.

### Findings and analysis errors

Example finding:

```text
Example.vi: warning [separate-compiled-code]: Enable separate compiled code from source (source-only setting).
```

A warning is reported but does not fail the default `error` threshold. Use
`--fail-on warning` or configure severity `error` when separation must gate CI.

Malformed input is an **analysis error**, not a disabled setting or a passing
check. Errors are written to stderr with the file path; other files are still
analyzed. Analysis errors always cause exit status 2, even with `--fail-on none`.

The initial reader supports RSRC format 3, creator `LBVW`, VI/control/template
types `LVIN`, `LVCC`, `sVIN`, and `sVCC`, with one uncompressed `LVSR` section
at index 0 and a save record of at least 68 bytes. Older/unknown formats,
missing or ambiguous save records, inconsistent headers, and out-of-bounds
records are errors. XML must be well-formed UTF-8 (a UTF-8 BOM is accepted);
wrong root elements, duplicate owner properties, wrong property types, and
invalid Boolean values are errors. The reader extracts this setting only; it
does not validate the contents of every unrelated RSRC resource.

## Adding a built-in rule

The [rule architecture decision](../decisions/0006-rule-runner-and-source-only-check.md)
describes the extension contract:

1. Add any necessary facts to the format layer, with bounded parsing and
   malformed-input tests. Reuse shared facts; do not read files inside a rule.
2. Implement `Rule` with a stable kebab-case ID, description, default severity,
   and a check that returns a message for violations.
3. Register the implementation in `BUILT_INS`. The runner handles enablement,
   severity overrides, finding paths, and failure thresholds.
4. Add focused Rust tests and document the rule here, including defaults,
   applicable types, options, limitations, and remediation.

Common configuration needs no factory. If a later rule needs additional
options, add a typed, validated rule-specific schema before constructing it;
do not accept arbitrary options or silently ignore unknown keys.
