# CLI: scanning folders

Build and run WireLint from the repository:

```text
cargo run -- scan path/to/labview/project
```

Or, after building or installing the executable:

```text
wirelint scan path/to/labview/project
```

The scan command recursively searches the specified folder and runs enabled
built-in analysis rules. Extension matching is case-insensitive. Files are read
without launching LabVIEW or modifying source files.

The initial candidate extensions are:

- `.vi`
- `.vit`
- `.vim`
- `.ctl`
- `.lvproj`
- `.lvclass`
- `.lvlib`

`.llb` archives are not included. Symbolic links encountered below the scan root
are skipped. Discovery uses extensions; the format readers then inspect file
contents for analysis.

## Rules and configuration

```text
wirelint rules
wirelint scan path/to/project --config checks.yaml
wirelint scan path/to/project --fail-on warning
```

All built-in rules are enabled by default, with their default severities. The
first rule checks that compiled code is separate from source. Its default
severity is `warning`, so findings do not fail the default `error` threshold.

`--config <FILE>` loads an explicit YAML file; there is no automatic discovery.
`--fail-on <error|warning|none>` overrides the YAML failure threshold. See
[configuration](configuration.md) and the [rule reference](rules.md).

## Output and exit statuses

The terminal report starts with the number of discovered files and a message
that enabled rules are running. It does not list every discovered path.
Findings and analysis errors still include their relative file paths, in sorted
file order within each group. Results are reported after analysis finishes;
the initial message is not a per-file progress indicator.

The final summary counts successfully analyzed files, findings, and analysis
errors. Files with format/read errors are not counted as analyzed. Errors go
to stderr, while the initial message, findings, and summary go to stdout. Other
files are still analyzed after individual file errors.

| Status | Meaning |
| --- | --- |
| `0` | No errors and no findings at or above the configured failure threshold |
| `1` | Findings meet or exceed the threshold |
| `2` | Command-line, configuration, discovery, output, or analysis error |

The scan folder must exist and be a directory. A folder with no matching files
prints a message and a zero-count summary and exits successfully, provided
configuration and output are valid. Unsupported binary formats are errors,
not passing checks. `--fail-on none` suppresses finding-based failures only,
not analysis errors.

Exclusions, GitLab Code Quality, and SARIF output are not implemented yet.
Global diagnostic logging, including discovered-path details, is also deferred;
see the [output and logging decision](../decisions/0007-concise-output-and-future-logging.md).
Use `wirelint --help` and `wirelint scan --help` for CLI help.
