# Release installers

- Status: Accepted
- Date: 2026-11-19

## Context

WireLint needs downloadable Windows MSI and Debian packages without requiring
Rust on users' machines. Distribution is limited to GitHub Releases. Release
automation should use existing tools rather than maintaining a bespoke
version validator, installer generator, and release orchestrator.

## Decision

Use cargo-dist, pinned in `dist-workspace.toml`, to generate the GitHub Actions
release workflow and WiX MSI definition, build x64 Windows and x86_64 GNU/Linux
archives, generate checksums, and publish GitHub Releases on version tags.

Use cargo-deb, pinned in `packaging/build-deb.sh`, through cargo-dist's supported
`extra-artifacts` build hook because cargo-dist does not natively generate
Debian packages. The global-artifact runner is Linux. The hook builds using
the shared `dist` profile and verifies the extracted package's executable and
notices without installing it on the build host.
The hook supplies a separate SHA-256 file for the extra Debian artifact.

Keep the Cargo package version as the source of truth. Maintainers update the
manifest and lockfile, commit them, and push a matching tag. Release CI does
not change versions or commit files. Initially document stable releases only,
within Windows Installer's version limits.

Run the existing Rust checks before local-artifact builds through cargo-dist's
build-setup extension. Use upload-mode PR validation so packaging changes are
built before release. Keep generated files under cargo-dist's management.

## Alternatives

- Direct WiX and cargo-deb workflows would permit modern WiX today, but require
  maintaining our own release orchestration and MSI definition.
- cargo-dist alone covers MSI and archives, but not the requested Debian package.
- APT repositories, crates.io publication, and additional package managers add
  distribution work outside the requested GitHub Releases scope.

## Consequences

- Most release logic is upstream-generated. Our only packaging build script is
  the Debian extra-artifact hook; no runtime Rust dependencies are added.
- cargo-dist's MSI backend currently uses the unsupported WiX 3 toolchain
  preinstalled on the Windows 2022 runner. Do not add a Chocolatey dependency
  pin for it: the initial release failed because the image already contained
  package revision `3.14.1.20250415`, which Chocolatey considers newer than
  the requested `3.14.1`.
  Accept this upstream limitation rather than forking its installer machinery;
  revisit it when upgrading cargo-dist. Stable upgrade/PATH GUIDs are retained
  in the Cargo metadata.
- Windows builds statically link the MSVC runtime using cargo-dist's default.
  The MSI includes the project license. Archives and Debian packages additionally
  include the existing third-party notices.
- Linux release builds use Ubuntu 22.04 and its glibc baseline. Local builds on
  newer distributions do not establish compatibility with that older baseline.
- Unsigned packages and public checksum files do not establish publisher
  identity. Signing and automatic updates remain separate future work.
- PR package builds cost more CI time than plan-only checks but exercise the
  installers before a tag is pushed.
