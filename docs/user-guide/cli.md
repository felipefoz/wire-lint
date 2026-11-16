# CLI: scanning folders

Build and run WireLint from the repository:

```text
cargo run -- scan path/to/labview/project
```

Or, after building or installing the executable:

```text
wirelint scan path/to/labview/project
```

The scan command recursively searches the specified folder and prints supported file paths relative to that folder, in sorted order. Extension matching is case-insensitive.

The initial candidate extensions are:

- `.vi`
- `.vit`
- `.vim`
- `.ctl`
- `.lvproj`
- `.lvclass`
- `.lvlib`

`.llb` archives are not included. Symbolic links encountered below the scan root are skipped. The scanner identifies candidates by extension only; it does not inspect file contents, run analysis rules, load configuration, or produce reports yet.

The scan folder must exist and be a directory. Filesystem errors are reported and return a nonzero exit status. A folder with no matching files prints a message and exits successfully.
