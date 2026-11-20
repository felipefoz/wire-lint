# Automated changelog and release tagging

- Status: Accepted
- Date: 2026-11-20

## Context

[0021-release-installers.md](0021-release-installers.md) established that
release CI never bumps versions or commits files, and that maintainers push a
tag manually once a version bump lands. We want release notes generated from
history rather than hand-written, and the project already requires
[Conventional Commits](../../CONTRIBUTING.md#commit-messages) for commit
subjects, which conventional changelog generators rely on to categorize
entries.

cargo-dist does not generate changelog content itself: it only reads the
section of a `CHANGELOG.md`/`RELEASES.md` matching the released tag's version
and uses it as the GitHub Release body. Something still has to write that
file and create the tag.

## Decision

Use [git-cliff](https://git-cliff.org/), configured in `cliff.toml`, to
generate `CHANGELOG.md` from Conventional Commits in Keep a Changelog format.
Base the configuration on git-cliff's own `keepachangelog` example
(https://github.com/orhun/git-cliff/blob/main/examples/keepachangelog.toml)
rather than a bespoke commit-grouping scheme, and add only the one rule that
example can't already cover: skipping the tool's own
`chore(release): ...` changelog-update commits, so a release doesn't list
itself as a change.

Add `.github/workflows/prepare-release.yml`, a hand-written workflow (not
cargo-dist-generated, so it is safe to edit directly) that runs on pushes to
`main` touching `Cargo.toml`. It reads the package version, and if no tag for
that version exists yet, regenerates `CHANGELOG.md` in full with git-cliff,
commits it, and pushes the `v<version>` tag. That tag push triggers the
existing cargo-dist release workflow unchanged.

This narrows, rather than reverses, 0021's "release CI does not change
versions or commit files": CI still never changes `[package].version` — that
stays a maintainer action — but a dedicated workflow now commits the derived
changelog and creates the tag once a version bump is pushed. Checking whether
the tag already exists makes repeated `Cargo.toml` edits (e.g. dependency
bumps) a no-op, so this only fires once per version.

The workflow pushes using a `RELEASE_PAT` repository secret (a fine-grained
PAT scoped to this repo with Contents: Read and write) instead of the default
`GITHUB_TOKEN`. GitHub does not let `GITHUB_TOKEN`-authored pushes trigger
other workflows, which would otherwise mean the pushed tag never starts the
release workflow.

## Alternatives

- Keep changelog writing fully manual: lowest automation, but drifts from the
  Conventional Commits the project already enforces and duplicates work
  already captured in commit history.
- Generate the changelog in `release-setup.yml` (cargo-dist's build-setup
  hook): that hook runs per-target inside the already-tagged release build,
  too late to add a commit the tag should point at, and it runs once per
  matrix entry rather than once per release.
- Have the release workflow itself (triggered by a manually pushed tag)
  generate and commit the changelog: the tag would then point at a commit
  that didn't exist when it was pushed, which published packages cannot
  reflect after the fact; tags must not move once published per 0021.
- Fully automate releases by triggering on every push to `main` (not just
  `Cargo.toml` changes): a natural next step once this narrower trigger is
  proven, but broader scope than currently requested.

## Consequences

- `CHANGELOG.md` is regenerated in full on every release; hand edits between
  releases are overwritten at the next version bump. Edit `cliff.toml`'s
  commit-grouping rules instead of editing generated entries.
- A new repository secret (`RELEASE_PAT`) exists with write access to this
  repo; it must be rotated/revoked like any other credential with push access.
- `main` currently has no branch protection, so a direct push from this
  workflow succeeds. If branch protection is added later, it must allow this
  workflow's pushes (e.g. an allow-listed actor) or this workflow will start
  failing.
- Maintainers no longer manually create and push release tags; pushing the
  version bump to `main` is now the only required manual release step.
