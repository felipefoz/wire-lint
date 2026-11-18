# Third-party notices and format references

## LabVIEW RSRC format research

WireLint's original Rust RSRC reader was informed by the
[pylabview project](https://github.com/mefistotelis/pylabview), specifically:

- [`LVrsrcontainer.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVrsrcontainer.py):
  format-3 RSRC headers, block-info tables, and resource file-type identifiers.
- [`LVblock.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVblock.py):
  block/section offsets and the LVSR save-record layout.
- [`LVinstrument.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVinstrument.py):
  the `viFlags2` word and `SourceOnly` bit 10, and the `instrState` word at
  LVSR offset 24 with `DebugCapable` bit 9.
- [`LVmisc.py`](https://github.com/mefistotelis/pylabview/blob/master/pylabview/LVmisc.py):
  LabVIEW version-word encoding and major/minor components.

These were consulted as descriptions of the file format, including a locally
vendored copy in labview-mcp, on 2026-11-17. No third-party implementation code,
substantial text, or binary fixtures are copied into WireLint. Its committed
RSRC test inputs are constructed synthetically. Existing external LabVIEW
fixtures were also checked read-only during development.

The VI description reader also uses pylabview's description of `STRG` resource
sections and LabVIEW string layouts in `LVblock.py` and `LVmisc.py`.

The debugging-status reader uses pylabview's `DebugCapable` definition.
[pylavi's `TypeLVSR`](https://github.com/marcpage/pylavi/blob/main/pylavi/resource_types.py)
and [validation logic](https://github.com/marcpage/pylavi/blob/main/pylavi/validate.py)
were also consulted on 2026-11-18. pylavi's debuggable mask additionally
includes instrument-state bit 30, which pylabview labels unknown. WireLint
uses only pylabview's documented bit 9. No implementation code was copied
from either project.

The `no-breakpoints` rule uses pylavi's `TypeLVSR.BREAKPOINTS_SET`
interpretation (word 5, mask `0x20000000`) from the same
[`resource_types.py`](https://github.com/marcpage/pylavi/blob/main/pylavi/resource_types.py)
reference. This is bit 29 of the instrument-state word, which pylabview leaves
unknown; WireLint follows pylavi's explicit breakpoint label rather than
deriving breakpoint presence from debugging capability. No implementation
code was copied.

pylavi's
[`TypeBDPW.has_password`](https://github.com/marcpage/pylavi/blob/main/pylavi/resource_types.py)
compares the first BDPW hash with the empty-password marker. WireLint uses
that documented interpretation to detect password presence without keeping
or exposing the hash. No implementation code was copied.

pylabview attributes these source files to Mefistotelis and distributes them
under the MIT License. Refer to its
[license](https://github.com/mefistotelis/pylabview/blob/master/LICENSE).

## Rust dependencies

WireLint uses Clap for the CLI, roxmltree for XML parsing, and Serde with
serde_yaml_ng for typed YAML configuration. Each dependency retains its own
license; dependency versions and transitive dependencies are recorded in
`Cargo.lock`.
