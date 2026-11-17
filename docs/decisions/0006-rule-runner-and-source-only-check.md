# Built-in rule runner and source-only check

- Status: Accepted
- Date: 2026-11-17

## Context

The initial scan only discovers candidates. Adding the first analysis test
requires an extension point for future checks, consistent findings, safe
LabVIEW format readers, configuration, and an explanation of test behavior.
The YAML and failure-threshold direction was established in
[decision 0004](0004-cli-configuration-and-reports.md).

## Decisions

- Call analysis tests **rules**, reserving "tests" for verification of WireLint.
- Separate discovery, format facts, rule evaluation, configuration, and CLI
  presentation into modules. Move CLI parsing to `src/cli.rs` now that it
  handles configuration and findings, as anticipated by decision 0004.
- Use a stateless `Rule` trait and a static built-in registry. Each rule owns
  its stable ID, description, default severity, and violation check. Read and
  parse each candidate once into shared facts, then run all enabled rules.
  Findings carry the relative path, ID, effective severity, and message.
- Do not add a factory hierarchy, plugin loader, or scripts. Common enablement
  and severity options do not require instance construction. If future checks
  need behavioral options, introduce typed, validated options and construct
  configured instances at the registry boundary when needed.
- Enable all built-ins by default. The first rule, `separate-compiled-code`,
  defaults to `warning` because separation is source-control policy, not proof
  of a broken VI. Keep the default failure threshold at `error`; make it
  independently configurable in YAML and overridable with `--fail-on`.
- Load configuration only from an explicit `--config` path. Use schema version
  1, common per-rule `enabled` and `severity` options, and reject unknown or
  duplicate fields/rule IDs. Defer discovery conventions until there is a
  concrete need, avoiding unexpected parent-directory configuration.
- Keep a `docs/wirelint.example.yaml` template listing every built-in
  rule with its default options. Test it against the registry so new checks
  cannot be omitted from the template. Users can copy it without configuration
  being loaded implicitly.
- Inspect each file's own source-only setting, covering all currently
  discovered extensions. Binary VI-family files use LVSR `viFlags2` bit 10;
  XML project/library/class documents use the root-owned
  `NI.LV.All.SourceOnly` Bool property. Missing XML properties mean false.
  Container defaults never substitute for member VI settings.
- Implement bounded, read-only RSRC format-3 access, not a general VI
  decompiler. Reject unsupported layouts, missing/ambiguous LVSR records, and
  malformed XML explicitly. Continue to other files, but never turn analysis
  errors into false properties or passing checks. Skip content reads only when
  no rules are enabled.
- Preserve the sorted candidate listing and add findings and a terminal
  summary. Return 0 for successful scans below the threshold, 1 for findings
  meeting it, and 2 for configuration, discovery, output, or analysis errors.
  Analysis errors are not suppressible with `fail-on: none`.
- Add `wirelint rules` to expose the registry's IDs, descriptions, and defaults.
- Use `roxmltree` for structural XML parsing and `serde` with `serde_yaml_ng`
  for typed YAML. These dependencies avoid brittle text matching and a
  handwritten YAML parser. Keep binary extraction dependency-free.
- Document usage and configuration in the user guide and format research in
  `THIRD_PARTY_NOTICES.md`. Exclusions and machine reports remain deferred;
  this change does not claim to implement all of decision 0004.

## Alternatives

- **Factory pattern now:** adds construction machinery without any
  rule-specific state. The registry leaves room for factories if typed
  behavioral options later justify them.
- **Each rule reads files itself:** duplicates I/O and parser work as checks
  grow, and couples checks to format details.
- **Look for a compiled-code block:** tests a byproduct rather than the stored
  setting and can misclassify files.
- **Find XML tags with string matching:** cannot reliably distinguish owner
  properties from comments, nested items, or escaped attributes.
- **Treat unsupported binaries as false or skip them silently:** reports
  unjustified policy violations or gives an incomplete scan a successful
  appearance.

## Consequences

New rules ordinarily change only the registry, their implementation, any
required facts, tests, and rule documentation. The runner is shared. The
initial shared-facts model is intentionally small, and more sophisticated
fact dependencies or parallel execution can be added when justified.

Scanning is now analysis, not discovery alone: empty or unsupported candidate
files that were previously listed successfully can produce status 2. Default
warning findings do not fail CI without a stricter threshold or severity
override. The format reader has an explicit supported subset rather than
claiming compatibility with every LabVIEW release.
