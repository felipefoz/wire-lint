//! End-to-end coverage of configuration, analysis, output, and process status.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        loop {
            let id = NEXT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("wirelint-cli-test-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("cannot create test directory: {error}"),
            }
        }
    }

    fn write(&self, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn scan(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wirelint"))
            .arg("scan")
            .arg(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn configured_scan(&self, config: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wirelint"))
            .arg("scan")
            .arg(&self.0)
            .arg("--config")
            .arg(config)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("cannot clean up test directory");
    }
}

/// Constructs a minimal format-3 VI with one LVSR save-record section.
fn vi_with_separation(enabled: bool, kind: &[u8; 4]) -> Vec<u8> {
    let mut bytes = vec![0; 192];
    for header in [0, 104] {
        bytes[header..header + 16].copy_from_slice(b"RSRC\r\n\0\x03LVINLBVW");
        bytes[header + 8..header + 12].copy_from_slice(kind);
    }
    let flags: u32 = if enabled { 1 << 10 } else { 0 };
    for (offset, value) in [
        (16, 104_u32),
        (20, 88),
        (24, 32),
        (28, 72),
        (32, 68),
        (36, 0x15008000),
        (44, flags),
        (120, 104),
        (124, 88),
        (128, 32),
        (132, 72),
        (144, 32),
        (148, 52),
        (152, 88),
        (168, 16),
        (176, u32::MAX),
    ] {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }
    bytes[160..164].copy_from_slice(b"LVSR");
    bytes
}

#[test]
fn scans_binary_flags_across_the_vi_family_without_modifying_files() {
    let directory = TestDirectory::new();
    let cases = [
        ("a.vi", false, b"LVIN"),
        ("b.VIT", true, b"sVIN"),
        ("c.vim", false, b"LVIN"),
        ("d.ctl", true, b"LVCC"),
    ];
    for (name, enabled, kind) in cases {
        directory.write(name, vi_with_separation(enabled, kind));
    }
    let output = directory.scan(&["--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Analyzed 4 file(s); 2 finding(s); 0 analysis error(s)."));
    for (name, enabled, kind) in cases {
        assert_eq!(
            stdout.contains(&format!("{name}: warning [separate-compiled-code]")),
            !enabled
        );
        assert_eq!(
            fs::read(directory.0.join(name)).unwrap(),
            vi_with_separation(enabled, kind)
        );
    }
}

#[test]
fn scans_xml_and_applies_yaml_severity_and_cli_threshold() {
    let directory = TestDirectory::new();
    directory.write("a.lvproj", "<Project/>");
    directory.write(
        "b.LVCLASS",
        r#"<LVClass><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></LVClass>"#,
    );
    directory.write(
        "c.lvlib",
        r#"<Library><Property Name="NI.LV.All.SourceOnly" Type="Bool">false</Property></Library>"#,
    );
    let output = directory.scan(&[]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        [
            "Found 3 supported LabVIEW file(s). Running enabled rules...",
            "a.lvproj: warning [separate-compiled-code]: Enable separate compiled code from source (source-only setting).",
            "c.lvlib: warning [separate-compiled-code]: Enable separate compiled code from source (source-only setting).",
            "Analyzed 3 file(s); 2 finding(s); 0 analysis error(s).",
        ]
    );
    assert!(stdout.contains("Analyzed 3 file(s); 2 finding(s); 0 analysis error(s)."));
    assert!(stdout.contains("a.lvproj: warning [separate-compiled-code]"));
    assert!(!stdout.contains("b.LVCLASS: warning"));
    assert!(output.stderr.is_empty());
    assert_eq!(
        directory.scan(&["--fail-on", "warning"]).status.code(),
        Some(1)
    );

    let config = directory.write(
        "checks.yaml",
        "version: 1\nfail-on: none\nrules:\n  separate-compiled-code:\n    severity: error\n",
    );
    assert_eq!(
        directory.configured_scan(&config, &[]).status.code(),
        Some(0)
    );
    let output = directory.configured_scan(&config, &["--fail-on", "error"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("a.lvproj: error [separate-compiled-code]")
    );
}

#[test]
fn analysis_errors_are_explicit_and_do_not_stop_other_files() {
    let directory = TestDirectory::new();
    directory.write("a.vi", b"not an RSRC file");
    directory.write("b.lvproj", "<Project/>");
    directory.write("c.lvlib", "<Library>");
    let output = directory.scan(&["--fail-on", "none"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("a.vi: analysis error:"));
    assert!(stderr.contains("c.lvlib: analysis error:"));
    assert!(stdout.contains("b.lvproj: warning [separate-compiled-code]"));
    assert!(stdout.contains("Analyzed 1 file(s); 1 finding(s); 2 analysis error(s)."));
}

#[test]
fn disabling_the_only_rule_skips_parsing() {
    let directory = TestDirectory::new();
    directory.write("broken.vi", []);
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    enabled: false\n",
    );
    let output = directory.configured_scan(&config, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Analyzed 0 file(s); 0 finding(s); 0 analysis error(s).")
    );
}

#[test]
fn invalid_or_missing_configuration_fails_before_scanning() {
    let directory = TestDirectory::new();
    for contents in [
        "rules:\n  unknown-rule: {}",
        "rules:\n  separate-compiled-code:\n    severity: fatal",
    ] {
        let config = directory.write("checks.yaml", contents);
        let output = directory.configured_scan(&config, &[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("configuration")
        );
    }
    let output = directory.configured_scan(&directory.0.join("missing.yaml"), &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("missing.yaml")
    );
}

#[test]
fn lists_rules_and_handles_an_empty_folder() {
    let output = Command::new(env!("CARGO_BIN_EXE_wirelint"))
        .arg("rules")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("separate-compiled-code [warning]")
    );
    let directory = TestDirectory::new();
    let output = directory.scan(&[]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("No supported LabVIEW files found.")
    );
}
