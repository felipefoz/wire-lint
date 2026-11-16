# WireLint

Static Analysis for LabVIEW.

WireLint aims to provide an independent static-analysis tool for LabVIEW files, similar in purpose to NI's VI Analyzer. It is not affiliated with, endorsed by, or sponsored by National Instruments (NI). LabVIEW is a trademark of NI.

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and contribution guidelines.

WireLint is licensed under the MIT License. See [LICENSE](LICENSE).

## Quick start

```text
cargo run -- scan path/to/labview/project
```

The scanner lists supported LabVIEW file candidates by extension. See the [CLI guide](docs/user-guide/cli.md).
