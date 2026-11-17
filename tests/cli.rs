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

/// Checks the variable elapsed-time line and returns the deterministic scan output.
fn scan_lines_without_timing(stdout: &str) -> Vec<&str> {
    let lines: Vec<_> = stdout.lines().collect();
    let timings: Vec<_> = lines
        .iter()
        .filter_map(|line| line.strip_prefix("Elapsed scan time: "))
        .collect();
    assert_eq!(timings.len(), 1, "{stdout}");
    let seconds: f64 = timings[0].strip_suffix(" s.").unwrap().parse().unwrap();
    assert!(seconds.is_finite() && seconds >= 0.0, "{stdout}");
    lines
        .into_iter()
        .filter(|line| !line.starts_with("Elapsed scan time: "))
        .collect()
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
        (36, 0x20008000),
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
    let output = directory.scan(&["--verbose", "--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    scan_lines_without_timing(&stdout);
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
    directory.write("a.lvproj", r#"<Project LVVersion="20008000"/>"#);
    directory.write(
        "b.LVCLASS",
        r#"<LVClass LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></LVClass>"#,
    );
    directory.write(
        "c.lvlib",
        r#"<Library LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">false</Property></Library>"#,
    );
    let output = directory.scan(&[]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let summary = scan_lines_without_timing(&stdout);
    assert_eq!(
        summary,
        [
            "Found 3 supported LabVIEW file(s). Running enabled rules...",
            "Analyzed 3 file(s); 2 finding(s); 0 analysis error(s).",
            "Findings by severity: 0 info; 2 warning; 0 error.",
            "Findings by rule:",
            "  separate-compiled-code: 2 finding(s).",
        ]
    );
    assert!(output.stderr.is_empty());
    for verbosity in [None, Some("--verbose")] {
        let mut args = vec!["--fail-on", "warning"];
        args.extend(verbosity);
        let output = directory.scan(&args);
        assert_eq!(output.status.code(), Some(1));
        if verbosity.is_some() {
            let verbose = String::from_utf8(output.stdout).unwrap();
            assert!(verbose.contains("a.lvproj: warning [separate-compiled-code]"));
            assert!(verbose.contains("c.lvlib: warning [separate-compiled-code]"));
            assert!(!verbose.contains("b.LVCLASS"));
            assert!(scan_lines_without_timing(&verbose).ends_with(&summary[1..]));
        } else {
            assert_eq!(
                scan_lines_without_timing(&String::from_utf8(output.stdout).unwrap()),
                summary
            );
        }
    }

    let config = directory.write(
        "checks.yaml",
        "version: 1\nfail-on: none\nrules:\n  separate-compiled-code:\n    severity: error\n",
    );
    assert_eq!(
        directory.configured_scan(&config, &[]).status.code(),
        Some(0)
    );
    for args in [
        vec!["--fail-on", "error"],
        vec!["--fail-on", "error", "--verbose"],
    ] {
        let output = directory.configured_scan(&config, &args);
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(stdout.contains("Findings by severity: 0 info; 0 warning; 2 error."));
        assert_eq!(
            stdout.contains("a.lvproj: error [separate-compiled-code]"),
            args.contains(&"--verbose")
        );
    }

    fs::write(
        &config,
        "rules:\n  separate-compiled-code:\n    severity: info\n",
    )
    .unwrap();
    for args in [
        vec!["--fail-on", "warning"],
        vec!["--fail-on", "warning", "--verbose"],
    ] {
        let output = directory.configured_scan(&config, &args);
        assert_eq!(output.status.code(), Some(0));
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .contains("Findings by severity: 2 info; 0 warning; 0 error.")
        );
    }
}

#[test]
fn analysis_errors_are_explicit_and_do_not_stop_other_files() {
    let directory = TestDirectory::new();
    directory.write("a.vi", b"not an RSRC file");
    directory.write("b.lvproj", r#"<Project LVVersion="20008000"/>"#);
    directory.write("c.lvlib", "<Library>");
    let output = directory.scan(&["--fail-on", "none"]);
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    scan_lines_without_timing(&stdout);
    assert!(stderr.contains("a.vi: analysis error:"));
    assert!(stderr.contains("c.lvlib: analysis error:"));
    assert!(!stdout.contains("b.lvproj"));
    assert!(stdout.contains("Analyzed 1 file(s); 1 finding(s); 2 analysis error(s)."));
    assert!(stdout.contains("Findings by severity: 0 info; 1 warning; 0 error."));
    let verbose = directory.scan(&["--verbose", "--fail-on", "none"]);
    assert_eq!(verbose.status.code(), Some(2));
    assert_eq!(String::from_utf8(verbose.stderr).unwrap(), stderr);
    assert!(
        String::from_utf8(verbose.stdout)
            .unwrap()
            .contains("b.lvproj: warning [separate-compiled-code]")
    );
}

#[test]
fn disabling_all_rules_skips_parsing() {
    let directory = TestDirectory::new();
    directory.write("broken.vi", []);
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    enabled: false\n  minimum-saved-version:\n    enabled: false\n",
    );
    let output = directory.configured_scan(&config, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        scan_lines_without_timing(&stdout),
        [
            "Found 1 supported LabVIEW file(s). Running enabled rules...",
            "Analyzed 0 file(s); 0 finding(s); 0 analysis error(s).",
        ]
    );
}

#[test]
fn passing_scan_omits_empty_breakdowns_in_both_modes() {
    let directory = TestDirectory::new();
    directory.write(
        "passing.lvproj",
        r#"<Project LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></Project>"#,
    );
    for args in [vec![], vec!["--verbose"]] {
        let output = directory.scan(&args);
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            scan_lines_without_timing(&stdout),
            [
                "Found 1 supported LabVIEW file(s). Running enabled rules...",
                "Analyzed 1 file(s); 0 finding(s); 0 analysis error(s).",
            ]
        );
    }
}

#[test]
fn template_excludes_git_and_labview_environments_before_analysis() {
    let directory = TestDirectory::new();
    let nested = directory.0.join("nested");
    fs::create_dir(&nested).unwrap();
    for parent in [&directory.0, &nested] {
        for name in [".git", ".lv-venv"] {
            let excluded = parent.join(name).join("deeper");
            fs::create_dir_all(&excluded).unwrap();
            fs::write(excluded.join("invalid.vi"), b"not an RSRC file").unwrap();
        }
    }
    fs::create_dir(directory.0.join(".git-backup")).unwrap();
    directory.write(
        "root.lvproj",
        r#"<Project LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></Project>"#,
    );
    fs::write(
        nested.join("nested.lvlib"),
        r#"<Library LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property></Library>"#,
    )
    .unwrap();
    fs::write(
        directory.0.join(".git-backup").join("kept.lvproj"),
        r#"<Project LVVersion="20008000"/>"#,
    )
    .unwrap();

    let template = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("wirelint.example.yaml");
    let output = directory.configured_scan(&template, &["--verbose"]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    assert!(output.stderr.is_empty());
    assert!(stdout.starts_with("Found 3 supported LabVIEW file(s)."));
    assert!(stdout.contains("Analyzed 3 file(s); 1 finding(s); 0 analysis error(s)."));
    assert!(stdout.contains(&format!(
        "{}: warning [separate-compiled-code]",
        Path::new(".git-backup").join("kept.lvproj").display()
    )));

    let output = directory.scan(&[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("Found 7 supported LabVIEW file(s).")
    );
    assert_eq!(
        String::from_utf8(output.stderr)
            .unwrap()
            .lines()
            .filter(|line| line.contains("analysis error:"))
            .count(),
        4
    );
}

#[test]
fn invalid_or_missing_configuration_fails_before_scanning() {
    let directory = TestDirectory::new();
    for contents in [
        "rules:\n  unknown-rule: {}",
        "rules:\n  separate-compiled-code:\n    severity: fatal",
        "exclude-folders: ['nested/.git']",
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
    let rules = String::from_utf8(output.stdout).unwrap();
    assert!(rules.contains("separate-compiled-code [warning]"));
    assert!(rules.contains("minimum-saved-version [warning]"));
    let directory = TestDirectory::new();
    for args in [vec![], vec!["--verbose"]] {
        let output = directory.scan(&args);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            scan_lines_without_timing(&stdout),
            [
                "No supported LabVIEW files found.",
                "Analyzed 0 file(s); 0 finding(s); 0 analysis error(s).",
            ]
        );
    }
}

#[test]
fn saved_version_rule_checks_files_and_respects_configuration() {
    let directory = TestDirectory::new();
    let mut older = vi_with_separation(true, b"LVIN");
    older[36..40].copy_from_slice(&0x19008000_u32.to_be_bytes());
    directory.write("older.vi", older);
    directory.write("equal.lvproj", r#"<Project LVVersion="20008000"/>"#);
    directory.write("newer.lvclass", r#"<LVClass LVVersion="21008000"/>"#);
    directory.write("old.lvlib", r#"<Library LVVersion="19008000"/>"#);
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    enabled: false\n",
    );
    let output = directory.configured_scan(&config, &["--verbose", "--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("older.vi: warning [minimum-saved-version]: Saved LabVIEW version 19.0 is below the minimum 20.0."));
    assert!(text.contains("old.lvlib: warning [minimum-saved-version]"));
    assert!(!text.contains("equal.lvproj:"));
    assert!(!text.contains("newer.lvclass:"));

    fs::write(&config, "rules:\n  separate-compiled-code:\n    enabled: false\n  minimum-saved-version:\n    minimum-version: '20.1'\n").unwrap();
    let output = directory.configured_scan(&config, &["--verbose"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("equal.lvproj: warning [minimum-saved-version]: Saved LabVIEW version 20.0 is below the minimum 20.1."));
    assert!(!text.contains("newer.lvclass:"));

    fs::write(&config, "rules:\n  separate-compiled-code:\n    enabled: false\n  minimum-saved-version:\n    minimum-version: '21.0'\n    severity: error\n").unwrap();
    let output = directory.configured_scan(&config, &["--verbose"]);
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("equal.lvproj: error [minimum-saved-version]: Saved LabVIEW version 20.0 is below the minimum 21.0."));
    assert!(!text.contains("newer.lvclass:"));

    directory.write("missing.lvproj", "<Project/>");
    let output = directory.configured_scan(&config, &[]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("missing root LVVersion")
    );
    fs::write(
        &config,
        "rules:\n  minimum-saved-version:\n    enabled: false\n",
    )
    .unwrap();
    let output = directory.configured_scan(&config, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}
