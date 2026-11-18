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

Findings contribute to the default summary totals. Use `--verbose` to show
individual findings, for example:

```text
Example.vi: warning [separate-compiled-code]: Enable separate compiled code from source (source-only setting).
```

A warning is counted but does not fail the default `error` threshold. Use
`--fail-on warning` or configure severity `error` when separation must gate CI.

Malformed input is an **analysis error**, not a disabled setting or a passing
check. Errors are written to stderr with the file path in both output modes;
other files are still analyzed. Analysis errors always cause exit status 2,
even with `--fail-on none`.

The initial reader supports RSRC format 3, creator `LBVW`, VI/control/template
types `LVIN`, `LVCC`, `sVIN`, and `sVCC`, with one uncompressed `LVSR` section
at index 0 and a save record of at least 68 bytes. Older/unknown formats,
missing or ambiguous save records, inconsistent headers, and out-of-bounds
records are errors. XML must be well-formed UTF-8 (a UTF-8 BOM is accepted);
wrong root elements, duplicate owner properties, wrong property types, and
invalid Boolean values are errors. The reader extracts this setting only; it
does not validate the contents of every unrelated RSRC resource.

## `file-description`

Requires a non-empty native description for each supported LabVIEW file. It
defaults to enabled with `warning` severity and applies to `.vi`, `.vit`,
`.vim`, `.ctl`, `.lvclass`, `.lvlib`, and `.lvproj`. Context Help is not
required.

The rule reads only the description belonging to the file itself:

- **VI-family files**: the first string in the `STRG` resource is the owning
  VI's description. An absent, empty, whitespace-only, or LabVIEW nonbreaking
  space-only description produces a finding.
- **Projects**: the direct owner property `NI.Project.Description`, with
  `Type="Str"`.
- **Libraries and classes**: the direct owner property `NI.Lib.Description`,
  with `Type="Str"`.

Missing XML properties and blank descriptions produce findings. Nested item
properties do not count. Duplicate or malformed owner properties and malformed
`STRG` records are analysis errors, not passing descriptions. Use
`--verbose` to show findings, for example:

```text
Example.vi: warning [file-description]: Add a non-empty description to this LabVIEW file.
```

The rule checks the native description field only; it does not inspect VI
Context Help or modify files.

Binary descriptions are read from the first string in an uncompressed
`STRG` section. The reader supports the corresponding string layouts for
pre-4.0 and 4.0-or-newer LabVIEW files; unsupported or malformed records are
analysis errors.

## `minimum-saved-version`

Requires each file's saved LabVIEW major.minor release to be at least the
configured minimum. It defaults to `20.0` and reports a `warning`. It applies
to `.vi`, `.vit`, `.vim`, `.ctl`, `.lvproj`, `.lvclass`, and `.lvlib`.

```yaml
rules:
  minimum-saved-version:
    enabled: true
    severity: warning
    minimum-version: '20.0'
```

Quote `minimum-version`: it must be a major.minor string, with a major from
0 to 99 and a minor from 0 to 9. Comparison is numeric, not decimal or
lexicographic. Patch, build, and prerelease stage are ignored. The check is
for a minimum, not a guarantee that an older LabVIEW can open a newer file.

For VI-family files, the version word is at offset 0 in the `LVSR` save-record
payload. For XML files, the root's `LVVersion` attribute encodes the same
version word as hexadecimal, potentially without leading zeroes. The first
three version digits encode the major and minor release. A missing or
malformed version is an analysis error while this rule is enabled; disabling
the rule skips version validation. Each file is checked independently, not
against its owning project, library, or class.

## `debugging-status`

Checks the stored **Allow debugging** setting for `.vi`, `.vit`, and `.vim`
files. The rule is enabled by default, reports `warning` findings, and flags
debugging when it is enabled.

```yaml
rules:
  debugging-status:
    enabled: true
    severity: warning
    fail-when: enabled
```

Set `fail-when: disabled` to flag VIs with debugging disabled instead.
This option accepts only `enabled` or `disabled`; it is separate from the
rule's Boolean `enabled` option. Findings follow the usual severity and
failure-threshold policy: default warnings do not fail a scan unless
`--fail-on warning` is used or the rule severity is changed to `error`.

The reader follows pylabview's `DebugCapable` definition: bit 9
(`0x00000200`) of the big-endian `instrState` word at offset 24 in the
`LVSR` payload. Other instrument-state bits do not enable debugging.
This checks the saved flag, not whether a debugging session is active.

VI and VI-template resource types (`LVIN`, `sVIN`) supply the flag. Controls
(`LVCC`, `sVCC`) and XML projects, libraries, and classes do not have an
applicable VI execution setting and produce no findings for this rule under
either policy. The existing RSRC format limitations and analysis-error
handling apply; malformed save records are not treated as debugging disabled.

Use `--verbose` to show findings:

```text
Example.vi: warning [debugging-status]: Debugging is enabled. Disable Allow debugging in VI Properties > Execution.
```

WireLint does not change the VI's settings.

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

Rule-specific options must be typed and validated; do not accept arbitrary
options or silently ignore unknown keys.
