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
YAML `exclude-folders` entries prune matching directories at any depth before
traversal. The [configuration template](../wirelint.example.yaml) excludes
`.git` and `.lv-venv`; load it with `--config` to apply those exclusions.

## Rules and configuration

```text
wirelint rules
wirelint scan path/to/project --config checks.yaml
wirelint scan path/to/project --fail-on warning
```

All built-in rules are enabled by default, with their default severities. The
rules check compiled-code separation and a minimum saved LabVIEW version
(default `20.0`). Both default to `warning`, so findings do not fail the
default `error` threshold.

Without `--config`, WireLint uses built-in defaults; no YAML file is required.
All rules are enabled, the failure threshold is `error`, and no folders are
excluded. `--config <FILE>` loads an explicit YAML file; there is no automatic
discovery.

`--fail-on <error|warning|none>` overrides the built-in or YAML failure
threshold, so it also works without `--config`. See
[configuration](configuration.md) and the [rule reference](rules.md).

## Output and exit statuses

The terminal report starts with the number of discovered files and a message
that enabled rules are running. It does not list every discovered path.
By default, findings are summarized by severity and rule, without individual
file paths or messages. Rule totals are sorted by rule ID; rules with no
findings are omitted. If there are no findings, both breakdowns are omitted.

For example:

```text
Found 120 supported LabVIEW file(s). Running enabled rules...
Analyzed 120 file(s); 35 finding(s); 0 analysis error(s).
Elapsed scan time: 0.123 s.
Findings by severity: 0 info; 35 warning; 0 error.
Findings by rule:
  separate-compiled-code: 35 finding(s).
```

Use `--verbose` (or `-v`) to include individual findings before the same
summary:

```text
wirelint scan path/to/project --verbose
```

Individual findings and analysis errors include their relative file paths,
in sorted file order within each group. Analysis errors remain visible in
both modes. Verbosity does not change analysis, counts, or exit status.
Results are reported after analysis finishes; the initial message is not a
per-file progress indicator.

The final summary counts successfully analyzed files, findings, and analysis
errors. Files with format/read errors are not counted as analyzed, and analysis
errors are separate from error-severity findings. Errors go to stderr, while
the initial message, verbose findings, and summary go to stdout. Other files
are still analyzed after individual file errors.

Elapsed scan time uses a monotonic clock and is printed in seconds to three
decimal places in both modes, including empty scans and scans with analysis
errors. Timing starts just before discovery and stops when analysis finishes;
it includes the initial status output, but excludes configuration loading and
final report rendering. A failure before analysis completes can prevent the
final summary. There are no per-file or per-rule timers.

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

General path/glob exclusions, command-line exclusion options, GitLab Code
Quality, SARIF output, report files, and a quiet mode are not implemented yet.
Global diagnostic logging, including discovered-path details and per-file
and per-rule elapsed timings, is also deferred; see the
[output and logging decision](../decisions/0007-concise-output-and-future-logging.md).
Use `wirelint --help` and `wirelint scan --help` for CLI help.
