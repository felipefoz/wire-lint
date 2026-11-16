# Contributing

Thanks for your interest in contributing to WireLint.

## Development setup

Install the stable Rust toolchain using [rustup](https://rustup.rs/), then confirm `rustc` and `cargo` are available:

```text
rustc --version
cargo --version
```

### Windows build tools

The default `x86_64-pc-windows-msvc` Rust target requires Microsoft's C++ build tools; Rustup does not install the MSVC linker or Windows SDK. Install the Build Tools bootstrapper with winget:

```powershell
winget install -e --id Microsoft.VisualStudio.BuildTools
```

The bootstrapper does not install the compiler toolchain by itself. After it finishes:

1. Open Visual Studio Installer and modify the Build Tools installation.
2. Select the **Desktop development with C++** workload.
3. Ensure the MSVC v143 build tools and a Windows 10/11 SDK are selected.
4. Apply the changes, then build from a Developer PowerShell or Developer Command Prompt.

## Build and checks

Run the relevant checks before submitting changes:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Add focused tests for new features and bug fixes.

## Commit messages

Use Conventional Commits for commit subjects:

```text
<type>[optional scope]: <summary>
```

For example: `feat(parser): add VI resource parsing`.

## Platform compatibility

WireLint targets both Windows and Linux. Prefer cross-platform Rust APIs, avoid assumptions about path separators, shells, or filesystem behavior, and isolate platform-specific code when it is unavoidable. For platform-sensitive changes, test on both operating systems when possible and state which platforms were validated.

## Design and documentation

Read relevant decision records in `docs/` before making design or architecture changes. Record new project, design, or architecture decisions in `docs/`, including context, alternatives considered, and consequences.

Keep changes focused and document user-visible behavior changes.
