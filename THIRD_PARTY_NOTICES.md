# Third-party notices and format references

## LabVIEW RSRC format research

WireLint's original Rust RSRC reader was informed by the
[pylabview project](https://github.com/mefistotelis/pylabview), specifically:

- [`LVrsrcontainer.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVrsrcontainer.py):
  format-3 RSRC headers, block-info tables, and resource file-type identifiers.
- [`LVblock.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVblock.py):
  block/section offsets and the LVSR save-record layout.
- [`LVinstrument.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVinstrument.py):
  the `viFlags2` word and `SourceOnly` bit 10.
- [`LVmisc.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVmisc.py):
  LabVIEW version-word encoding and major/minor components.

These were consulted as descriptions of the file format, including a locally
vendored copy in labview-mcp, on 2026-11-17. No third-party implementation code,
substantial text, or binary fixtures are copied into WireLint. Its committed
RSRC test inputs are constructed synthetically. Existing external LabVIEW
fixtures were also checked read-only during development.

pylabview attributes these source files to Mefistotelis and distributes them
under the MIT License. Refer to its
[license](https://github.com/mefistotelis/pylabview/blob/master/LICENSE).

## Rust dependencies

WireLint uses Clap for the CLI, roxmltree for XML parsing, and Serde with
serde_yaml_ng for typed YAML configuration. Each dependency retains its own
license; dependency versions and transitive dependencies are recorded in
`Cargo.lock`.
