# Finding summaries, verbosity, and elapsed scan time

- Status: Accepted
- Date: 2026-11-17

## Context

Individual findings can still overwhelm the terminal after removing the
discovered-path listing in decision 0007. Users need concise totals and a way
to request details. Elapsed time helps users assess scan duration.

## Decisions

- Show analyzed-file, finding, and analysis-error totals by default. When
  findings exist, add totals for every severity and for each rule with findings.
  Sort rule totals by ID for deterministic output as the registry grows.
- Add `--verbose` (`-v`) to include individual findings before the same
  summary. This changes presentation only; rule execution and exit thresholds
  are unchanged. Keep this option out of YAML.
- Always show analysis errors with file paths on stderr. Keep finding
  severities separate from analysis-error counts.
- Include elapsed scan time in completed scan summaries in both modes,
  even with no files, no enabled rules, or analysis errors. Use the standard
  library's monotonic `Instant` clock and print seconds to three decimal places.
  Measure from just before discovery through completion of analysis,
  including the initial status output, but excluding configuration loading
  and final report rendering. Early failures may prevent the final summary.
- This supersedes decision 0007's always-visible individual findings.
  Application-wide diagnostic logging remains deferred; its future scope
  includes discovered-path details and per-file and per-rule elapsed timings,
  as recorded in [decision 0007](0007-concise-output-and-future-logging.md).
  Quiet mode and machine-readable report files also remain deferred.
  No new dependencies are needed.

## Alternatives and consequences

Reports-only output would hide useful feedback until report formats and
destinations exist. Summary output keeps interactive scans useful without
printing one line per finding.

A live timer or progress display would require terminal updates and a separate
noninteractive output policy. A final elapsed duration works with redirected
output and CI logs. It cannot identify a stalled file or distinguish discovery
cost from analysis cost.

The runner still collects all findings before presentation. This change
reduces terminal output, not analysis work or memory usage. Consumers that
parsed individual terminal findings must use `--verbose` until a structured
report format is available.
