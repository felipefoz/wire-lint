# Concise scan output and future diagnostic logging

- Status: Accepted
- Date: 2026-11-17

## Context

Printing every discovered path floods the terminal on large scans. Analysis
then runs without output until the report is ready, which can look like a hang.
The discovery listing was retained by
[decision 0006](0006-rule-runner-and-source-only-check.md).

## Decisions

- Replace the discovered-path listing with the file count and a message that
  enabled rules are running. Flush this message before starting analysis.
- Keep file paths in findings and analysis errors, and retain the final summary
  and existing failure thresholds. This supersedes the candidate-listing
  behavior in decisions 0004 and 0006.
- Record global, application-wide diagnostic logging as future work. It should
  provide a place for discovered paths and analysis diagnostics without adding
  those details to normal terminal output. Log levels, destinations, and
  configuration remain to be designed; no logging options are implemented now.

## Alternatives and consequences

Keeping the path listing would preserve the old output but retain the noise.
Adding a verbose flag now would introduce a separate diagnostic control before
the global logging design exists.

Normal output is smaller. Results are still printed after analysis completes;
the start message confirms the phase but does not measure per-file progress
or diagnose an actual stalled read or parser.
