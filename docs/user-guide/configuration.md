# Rule configuration

Without a configuration file, WireLint enables every built-in rule with its
default severity, uses the `error` failure threshold, and excludes no folders.

Load YAML explicitly:

```text
wirelint scan path/to/project --config checks.yaml
```

There is no automatic configuration-file discovery. The configuration path is
relative to the current working directory, not the scanned folder.

## Configuration template

Copy [`wirelint.example.yaml`](../wirelint.example.yaml) as a starting point,
or use it directly from the repository root:

```text
wirelint scan path/to/project --config docs/wirelint.example.yaml
```

The template lists every built-in rule with its default options. Add new rules
there as they are implemented; a Rust test checks that the template remains
complete and matches the defaults. Adjust `enabled` and `severity` in your copy
to select the checks and levels for your project.
It also excludes folders named `.git` and `.lv-venv` at any depth.

## Schema

```yaml
version: 1
fail-on: error
exclude-folders:
  - .git
  - .lv-venv
rules:
  separate-compiled-code:
    enabled: true
    severity: warning
  minimum-saved-version:
    enabled: true
    severity: warning
    minimum-version: '20.0'
```

| Key | Accepted values | Default |
| --- | --- | --- |
| `version` | `1` | `1` |
| `fail-on` | `error`, `warning`, `none` | `error` |
| `exclude-folders` | List of literal folder names | `[]` (no exclusions) |
| `rules` | Mapping of built-in rule IDs to option mappings | No overrides |
| `rules.<id>.enabled` | YAML Boolean | `true` |
| `rules.<id>.severity` | `info`, `warning`, `error` | Rule default |
| `rules.minimum-saved-version.minimum-version` | Quoted `major.minor` string | `'20.0'` |

Fields may be omitted. An omitted rule remains enabled; `rules: {}` does not
disable built-ins. An empty options mapping also retains defaults. Use `{}`
for a configuration with no overrides.

Unknown rule IDs, unknown options, duplicate keys, unsupported schema versions,
invalid YAML, and invalid option values are errors. WireLint reports the
configuration path and exits with status 2 before scanning. This version
supports a single YAML document; custom scripts are not supported. Rule-specific
options are accepted only for the rule that defines them.

## Folder exclusions

Use `exclude-folders` to skip entire directory trees during discovery:

```yaml
exclude-folders:
  - .git
  - .lv-venv
```

Each entry matches an exact folder name, case-sensitively on both Windows and
Linux, at any depth below the scan root. For example, `.git` skips both
`.git` and `nested/.git`, but not `.git-backup` or `.GIT`. Excluded folders are
not traversed, and their files are not counted, read, or analyzed.

Entries are names, not paths or glob patterns. Empty/blank names, `.` and `..`,
NUL characters, and names containing either path separator (`/` or `\`) are
configuration errors. The scan root itself is always scanned, even if its name
matches an exclusion. Exclusions affect directories only, not similarly named
files.

Omitting the field or using `exclude-folders: []` keeps every folder eligible
for scanning. The `.git` and `.lv-venv` exclusions apply only when loading the
template or another configuration that lists them; the template is not loaded
automatically. Use `--config` as shown above. Command-line exclusion options
and exclusion-list files are not implemented yet.

## Enablement and severity

To disable the first rule:

```yaml
rules:
  separate-compiled-code:
    enabled: false
```

To make a violation an error:

```yaml
rules:
  separate-compiled-code:
    severity: error
```

Severity controls the finding's reported level. `fail-on` independently
controls whether those findings fail the scan:

- `error`: fail for error findings.
- `warning`: fail for warning or error findings.
- `none`: never fail because of findings.

Info findings do not fail any available threshold. Configuration, discovery,
file-read, and format errors always fail, independently of this threshold.
The default summary and `--verbose` mode count the same findings and use the
same threshold; verbosity is a CLI presentation option, not a YAML setting.

The command-line threshold overrides YAML:

```text
wirelint scan path/to/project --config checks.yaml --fail-on warning
```

If all rules are disabled, discovery still runs, but file contents are not read
and the analyzed count is zero. No content validation is claimed in this mode.

See the [rule reference](rules.md) for each rule's defaults and interpretation,
and the [CLI guide](cli.md) for output and exit statuses. General path/glob
exclusions and machine report formats are not implemented in this version.
