# Agent instructions

- Keep changes focused and follow existing Rust style.
- Add focused tests for every new feature.
- Run `cargo fmt --check` and `cargo test` for Rust changes.
- Run `cargo clippy --all-targets --all-features -- -D warnings` when practical.
- Use Conventional Commits for commit subjects: `<type>[optional scope]: <summary>`.
- Keep the project compatible with Windows and Linux. Avoid platform-specific assumptions; validate platform-sensitive changes on both when possible.
- Avoid adding dependencies unless they are needed; explain why when you do.
- Read relevant `docs/` decision records before design or implementation work.
- For VI/RSRC binary-format work, consult the references and attribution guidance in `docs/decisions/0001-project-identity-and-format-references.md`.
- Record every project, design, or architecture decision in `docs/`, including context, alternatives, and consequences.
