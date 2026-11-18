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

/// Constructs a format-3 VI with LVSR and, by default, a nonempty STRG description.
fn vi_with_separation(enabled: bool, kind: &[u8; 4]) -> Vec<u8> {
    vi_with_description(enabled, kind, Some(b"Fixture VI description"))
}

fn vi_without_description(enabled: bool, kind: &[u8; 4]) -> Vec<u8> {
    vi_with_description(enabled, kind, None)
}

fn vi_with_description(enabled: bool, kind: &[u8; 4], description: Option<&[u8]>) -> Vec<u8> {
    let block_count = if description.is_some() { 2 } else { 1 };
    let data_size = 72 + description.map_or(0, |text| 8 + text.len());
    let table_size = 4 + block_count * 12;
    let info_size = 52 + table_size + block_count * 20;
    let info_offset = 32 + data_size;
    let mut bytes = vec![0; info_offset + info_size];
    for header in [0, info_offset] {
        bytes[header..header + 16].copy_from_slice(b"RSRC\r\n\0\x03LVINLBVW");
        bytes[header + 8..header + 12].copy_from_slice(kind);
        put_u32(&mut bytes, header + 16, info_offset as u32);
        put_u32(&mut bytes, header + 20, info_size as u32);
        put_u32(&mut bytes, header + 24, 32);
        put_u32(&mut bytes, header + 28, data_size as u32);
    }
    let flags: u32 = if enabled { 1 << 10 } else { 0 };
    put_u32(&mut bytes, 32, 68);
    put_u32(&mut bytes, 36, 0x20008000);
    put_u32(&mut bytes, 44, flags);

    put_u32(&mut bytes, info_offset + 40, 32);
    put_u32(&mut bytes, info_offset + 44, 52);
    put_u32(&mut bytes, info_offset + 48, info_size as u32);
    let table = info_offset + 52;
    put_u32(&mut bytes, table, (block_count - 1) as u32);
    for index in 0..block_count {
        let entry = table + 4 + index * 12;
        let section = table + table_size + index * 20;
        let id = if index == 0 { b"LVSR" } else { b"STRG" };
        bytes[entry..entry + 4].copy_from_slice(id);
        put_u32(&mut bytes, entry + 8, (table_size + index * 20) as u32);
        put_u32(&mut bytes, section + 4, u32::MAX);
        if index == 1 {
            put_u32(&mut bytes, section + 12, 72);
        }
    }
    if let Some(description) = description {
        let record = 32 + 72;
        put_u32(&mut bytes, record, (4 + description.len()) as u32);
        put_u32(&mut bytes, record + 4, description.len() as u32);
        bytes[record + 8..record + 8 + description.len()].copy_from_slice(description);
    }
    bytes
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
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
fn debugging_status_defaults_to_enabled_and_supports_reversed_or_disabled_policy() {
    let directory = TestDirectory::new();
    for (extension, kind) in [("VI", b"LVIN"), ("vit", b"sVIN"), ("vim", b"LVIN")] {
        for enabled in [false, true] {
            let mut bytes = vi_with_separation(true, kind);
            put_u32(&mut bytes, 36 + 24, if enabled { 1 << 9 } else { 0 });
            directory.write(&format!("{enabled}.{extension}"), bytes);
        }
    }
    let mut control = vi_with_separation(true, b"LVCC");
    put_u32(&mut control, 36 + 24, 1 << 9);
    directory.write("ignored.ctl", control);
    for (extension, root) in [
        ("lvproj", "Project"),
        ("lvlib", "Library"),
        ("lvclass", "LVClass"),
    ] {
        let property = if root == "Project" {
            "NI.Project.Description"
        } else {
            "NI.Lib.Description"
        };
        directory.write(
            &format!("ignored.{extension}"),
            format!(r#"<{root} LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property><Property Name="{property}" Type="Str">Description</Property></{root}>"#),
        );
    }

    for (args, status) in [
        (vec!["--verbose"], 0),
        (vec!["--verbose", "--fail-on", "warning"], 1),
    ] {
        let output = directory.scan(&args);
        assert_eq!(output.status.code(), Some(status));
        assert!(output.stderr.is_empty());
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            stdout.contains("Analyzed 10 file(s); 3 finding(s); 0 analysis error(s)."),
            "{stdout}"
        );
        for extension in ["VI", "vit", "vim"] {
            assert!(stdout.contains(&format!(
                "true.{extension}: warning [debugging-status]: Debugging is enabled. Disable"
            )));
            assert!(!stdout.contains(&format!("false.{extension}:")));
        }
        assert!(!stdout.contains("ignored."));
    }

    let config = directory.write(
        "checks.yaml",
        "rules:\n  debugging-status:\n    fail-when: disabled\n    severity: error\n",
    );
    let report = directory.0.join("report.json");
    let output = directory.configured_scan(
        &config,
        &[
            "--verbose",
            "--format",
            "sarif",
            "--output",
            report.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Analyzed 10 file(s); 3 finding(s); 0 analysis error(s)."));
    for extension in ["VI", "vit", "vim"] {
        assert!(stdout.contains(&format!(
            "false.{extension}: error [debugging-status]: Debugging is disabled. Enable"
        )));
        assert!(!stdout.contains(&format!("true.{extension}:")));
    }
    assert!(!stdout.contains("ignored."));
    let json: serde_json::Value = serde_json::from_slice(&fs::read(report).unwrap()).unwrap();
    let results = json["runs"][0]["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);
    assert!(
        results
            .iter()
            .all(|finding| finding["ruleId"] == "debugging-status" && finding["level"] == "error")
    );

    fs::write(&config, "rules:\n  debugging-status:\n    enabled: false\n").unwrap();
    let output = directory.configured_scan(&config, &["--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Analyzed 10 file(s); 0 finding(s); 0 analysis error(s)."));
}

#[test]
fn breakpoints_fail_by_default_but_controls_are_not_checked() {
    let directory = TestDirectory::new();
    let mut breakpoint_vi = vi_with_separation(true, b"LVIN");
    put_u32(&mut breakpoint_vi, 36 + 24, 0x2000_0000);
    directory.write("with-breakpoint.vi", breakpoint_vi);
    directory.write("without-breakpoint.vi", vi_with_separation(true, b"LVIN"));
    let mut breakpoint_control = vi_with_separation(true, b"LVCC");
    put_u32(&mut breakpoint_control, 36 + 24, 0x2000_0000);
    directory.write("control.ctl", breakpoint_control);

    let output = directory.scan(&["--verbose"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Analyzed 3 file(s); 1 finding(s); 0 analysis error(s)."));
    assert!(stdout.contains(
        "with-breakpoint.vi: error [no-breakpoints]: Remove all block diagram breakpoints before committing this VI."
    ));
    assert!(!stdout.contains("without-breakpoint.vi:"));
    assert!(!stdout.contains("control.ctl:"));
}

#[test]
fn scans_xml_and_applies_yaml_severity_and_cli_threshold() {
    let directory = TestDirectory::new();
    directory.write(
        "a.lvproj",
        r#"<Project LVVersion="20008000"><Property Name="NI.Project.Description" Type="Str">Project documentation</Property></Project>"#,
    );
    directory.write(
        "b.LVCLASS",
        r#"<LVClass LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property><Property Name="NI.Lib.Description" Type="Str">Class documentation</Property></LVClass>"#,
    );
    directory.write(
        "c.lvlib",
        r#"<Library LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">false</Property><Property Name="NI.Lib.Description" Type="Str">Library documentation</Property></Library>"#,
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
fn file_description_checks_every_supported_labview_extension() {
    let directory = TestDirectory::new();
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    enabled: false\n  saved-version:\n    enabled: false\n  file-description:\n    enabled: true\n",
    );
    for (extension, kind) in [
        ("vi", b"LVIN"),
        ("vit", b"sVIN"),
        ("vim", b"LVIN"),
        ("ctl", b"LVCC"),
    ] {
        directory.write(
            &format!("documented.{extension}"),
            vi_with_description(true, kind, Some(b"File description")),
        );
        directory.write(
            &format!("undocumented.{extension}"),
            vi_without_description(true, kind),
        );
    }
    for (extension, root, property) in [
        ("lvproj", "Project", "NI.Project.Description"),
        ("lvclass", "LVClass", "NI.Lib.Description"),
        ("lvlib", "Library", "NI.Lib.Description"),
    ] {
        directory.write(
            &format!("documented.{extension}"),
            format!("<{root}><Property Name=\"{property}\" Type=\"Str\">File description</Property></{root}>"),
        );
        directory.write(&format!("undocumented.{extension}"), format!("<{root}/>"));
    }

    let output = directory.configured_scan(&config, &["--verbose", "--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Analyzed 14 file(s); 7 finding(s); 0 analysis error(s)."));
    for extension in ["vi", "vit", "vim", "ctl", "lvproj", "lvclass", "lvlib"] {
        assert!(
            stdout.contains(&format!(
                "undocumented.{extension}: warning [file-description]"
            )),
            "{stdout}"
        );
        assert!(
            !stdout
                .lines()
                .any(|line| line.starts_with(&format!("documented.{extension}:"))),
            "{stdout}"
        );
    }
}

#[test]
fn analysis_errors_are_explicit_and_do_not_stop_other_files() {
    let directory = TestDirectory::new();
    directory.write("a.vi", b"not an RSRC file");
    directory.write(
        "b.lvproj",
        r#"<Project LVVersion="20008000"><Property Name="NI.Project.Description" Type="Str">Project documentation</Property></Project>"#,
    );
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
        "rules:\n  separate-compiled-code:\n    enabled: false\n  saved-version:\n    enabled: false\n  file-description:\n    enabled: false\n  debugging-status:\n    enabled: false\n  no-breakpoints:\n    enabled: false\n",
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
        r#"<Project LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property><Property Name="NI.Project.Description" Type="Str">Project documentation</Property></Project>"#,
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
        r#"<Project LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property><Property Name="NI.Project.Description" Type="Str">Project documentation</Property></Project>"#,
    );
    fs::write(
        nested.join("nested.lvlib"),
        r#"<Library LVVersion="20008000"><Property Name="NI.LV.All.SourceOnly" Type="Bool">true</Property><Property Name="NI.Lib.Description" Type="Str">Library documentation</Property></Library>"#,
    )
    .unwrap();
    fs::write(
        directory.0.join(".git-backup").join("kept.lvproj"),
        r#"<Project LVVersion="20008000"><Property Name="NI.Project.Description" Type="Str">Project documentation</Property></Project>"#,
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
    assert!(rules.contains("saved-version [warning]"));
    assert!(!rules.contains("minimum-saved-version"));
    assert!(rules.contains("file-description [warning]"));
    assert!(rules.contains("debugging-status [warning]"));
    assert!(rules.contains("no-breakpoints [error]"));
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
        "rules:\n  separate-compiled-code:\n    enabled: false\n  file-description:\n    enabled: false\n",
    );
    let output = directory.configured_scan(&config, &["--verbose", "--fail-on", "warning"]);
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(
        "older.vi: warning [saved-version]: Saved LabVIEW version 19.0 is below the minimum 20.0."
    ));
    assert!(text.contains("old.lvlib: warning [saved-version]"));
    assert!(!text.contains("equal.lvproj:"));
    assert!(!text.contains("newer.lvclass:"));

    fs::write(&config, "rules:\n  separate-compiled-code:\n    enabled: false\n  file-description:\n    enabled: false\n  saved-version:\n    minimum-version: '20.1'\n").unwrap();
    let output = directory.configured_scan(&config, &["--verbose"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("equal.lvproj: warning [saved-version]: Saved LabVIEW version 20.0 is below the minimum 20.1."));
    assert!(!text.contains("newer.lvclass:"));

    fs::write(&config, "rules:\n  separate-compiled-code:\n    enabled: false\n  file-description:\n    enabled: false\n  saved-version:\n    minimum-version: '21.0'\n    severity: error\n").unwrap();
    let output = directory.configured_scan(&config, &["--verbose"]);
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(
        "equal.lvproj: error [saved-version]: Saved LabVIEW version 20.0 is below the minimum 21.0."
    ));
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
        "rules:\n  saved-version:\n    enabled: false\n  file-description:\n    enabled: false\n",
    )
    .unwrap();
    let output = directory.configured_scan(&config, &[]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
}

#[test]
fn saved_version_bounds_apply_to_binary_and_xml_files() {
    let directory = TestDirectory::new();
    for (name, word) in [
        ("older.vi", 0x19008000_u32),
        ("equal.vi", 0x20008000_u32),
        ("newer.vi", 0x21008000_u32),
    ] {
        let mut bytes = vi_with_separation(true, b"LVIN");
        bytes[36..40].copy_from_slice(&word.to_be_bytes());
        directory.write(name, bytes);
    }
    directory.write("older.lvproj", r#"<Project LVVersion="19008000"/>"#);
    directory.write("equal.lvclass", r#"<LVClass LVVersion="20008000"/>"#);
    directory.write("newer.lvlib", r#"<Library LVVersion="21008000"/>"#);
    for (bounds, expected_count) in [
        ("maximum-version: '20.0'", 2),
        ("minimum-version: '20.0'\n    maximum-version: '20.0'", 4),
    ] {
        let config = directory.write(
            "checks.yaml",
            format!(
                "rules:\n  separate-compiled-code:\n    enabled: false\n  file-description:\n    enabled: false\n  debugging-status:\n    enabled: false\n  saved-version:\n    {bounds}\n"
            ),
        );
        let output = directory.configured_scan(&config, &["--verbose", "--fail-on", "warning"]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("newer.vi: warning [saved-version]: Saved LabVIEW version 21.0 is above the maximum 20.0."));
        assert!(text.contains("newer.lvlib: warning [saved-version]: Saved LabVIEW version 21.0 is above the maximum 20.0."));
        assert!(!text.contains("equal.vi:"));
        assert!(!text.contains("equal.lvclass:"));
        assert_eq!(text.matches("[saved-version]").count(), expected_count);
        assert_eq!(text.contains("older.vi:"), expected_count == 4);
        assert_eq!(text.contains("older.lvproj:"), expected_count == 4);
    }
}

#[test]
fn saved_version_configuration_errors_prevent_scanning() {
    let directory = TestDirectory::new();
    directory.write("unreadable.vi", []);
    for (rule, options, expected) in [
        (
            "minimum-saved-version",
            "enabled: false",
            "renamed to 'saved-version'",
        ),
        (
            "saved-version",
            "minimum-version: '21.0'\n    maximum-version: '20.0'",
            "minimum-version must not exceed maximum-version",
        ),
    ] {
        let config = directory.write("checks.yaml", format!("rules:\n  {rule}:\n    {options}\n"));
        let output = directory.configured_scan(&config, &[]);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("checks.yaml"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn ci_reports_preserve_terminal_output_thresholds_and_stable_fingerprints() {
    let directory = TestDirectory::new();
    let nested = directory.0.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(
        nested.join("example #.vi"),
        vi_with_separation(false, b"LVIN"),
    )
    .unwrap();
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    severity: error\n",
    );
    let destination = directory.0.join("report.json");
    let destination_arg = destination.to_str().unwrap();
    let baseline = directory.configured_scan(&config, &["--verbose"]);
    for (format, threshold, status) in [
        ("gitlab", "error", 1),
        ("sarif", "error", 1),
        ("gitlab", "none", 0),
        ("sarif", "none", 0),
    ] {
        let output = directory.configured_scan(
            &config,
            &[
                "--verbose",
                "--fail-on",
                threshold,
                "--format",
                format,
                "--output",
                destination_arg,
            ],
        );
        assert_eq!(output.status.code(), Some(status));
        assert!(output.stderr.is_empty());
        assert_eq!(
            scan_lines_without_timing(std::str::from_utf8(&output.stdout).unwrap()),
            scan_lines_without_timing(std::str::from_utf8(&baseline.stdout).unwrap())
        );
        let bytes = fs::read(&destination).unwrap();
        assert!(!bytes.starts_with(&[0xEF, 0xBB, 0xBF]));
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        if format == "gitlab" {
            assert_eq!(json.as_array().unwrap().len(), 1);
            assert_eq!(json[0]["location"]["path"], "nested/example #.vi");
            assert_eq!(json[0]["location"]["lines"]["begin"], 1);
            assert_eq!(json[0]["severity"], "major");
        } else {
            let run = &json["runs"][0];
            assert_eq!(run["results"].as_array().unwrap().len(), 1);
            assert_eq!(run["results"][0]["level"], "error");
            assert_eq!(run["invocations"][0]["executionSuccessful"], true);
            assert_eq!(
                run["results"][0]["locations"][0]["physicalLocation"],
                serde_json::json!({
                    "artifactLocation": { "uri": "nested/example%20%23.vi" }
                })
            );
        }
        let repeated =
            directory.configured_scan(&config, &["--format", format, "--output", destination_arg]);
        assert_eq!(repeated.status.code(), Some(1));
        assert_eq!(fs::read(&destination).unwrap(), bytes);
    }
}

#[test]
fn ci_reports_cover_empty_scans_disabled_rules_and_partial_analysis_failures() {
    let directory = TestDirectory::new();
    let destination = directory.0.join("report.json");
    for format in ["gitlab", "sarif"] {
        let args = [
            "--format",
            format,
            "--output",
            destination.to_str().unwrap(),
        ];
        let output = directory.scan(&args);
        assert_eq!(output.status.code(), Some(0));
        let empty: serde_json::Value =
            serde_json::from_slice(&fs::read(&destination).unwrap()).unwrap();
        if format == "gitlab" {
            assert_eq!(empty, serde_json::json!([]));
        } else {
            assert_eq!(empty["runs"][0]["results"], serde_json::json!([]));
            assert_eq!(
                empty["runs"][0]["invocations"][0]["executionSuccessful"],
                true
            );
        }
    }
    directory.write("broken.vi", b"not an RSRC file");
    directory.write(
        "finding.lvproj",
        r#"<Project LVVersion="20008000"><Property Name="NI.Project.Description" Type="Str">Description</Property></Project>"#,
    );
    for format in ["gitlab", "sarif"] {
        let output = directory.scan(&[
            "--format",
            format,
            "--output",
            destination.to_str().unwrap(),
            "--fail-on",
            "none",
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(
            std::str::from_utf8(&output.stderr)
                .unwrap()
                .contains("broken.vi: analysis error:")
        );
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(&destination).unwrap()).unwrap();
        if format == "gitlab" {
            assert_eq!(json.as_array().unwrap().len(), 1);
            assert_eq!(json[0]["location"]["path"], "finding.lvproj");
        } else {
            let run = &json["runs"][0];
            assert_eq!(run["results"].as_array().unwrap().len(), 1);
            assert_eq!(run["invocations"][0]["executionSuccessful"], false);
            assert_eq!(
                run["invocations"][0]["toolExecutionNotifications"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
    }
    let config = directory.write(
        "checks.yaml",
        "rules:\n  separate-compiled-code:\n    enabled: false\n  saved-version:\n    enabled: false\n  file-description:\n    enabled: false\n  debugging-status:\n    enabled: false\n  no-breakpoints:\n    enabled: false\n",
    );
    let output = directory.configured_scan(
        &config,
        &[
            "--format",
            "sarif",
            "--output",
            destination.to_str().unwrap(),
        ],
    );
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&destination).unwrap()).unwrap();
    assert_eq!(json["runs"][0]["results"], serde_json::json!([]));
    assert_eq!(
        json["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
}

#[test]
fn ci_report_options_and_write_errors_fail_explicitly() {
    let directory = TestDirectory::new();
    for args in [
        vec!["--format", "gitlab"],
        vec!["--output", "report.json"],
        vec!["--format", "unknown", "--output", "report.json"],
    ] {
        let output = directory.scan(&args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    for destination in [
        directory.0.join("missing").join("report.json"),
        directory.0.clone(),
    ] {
        let output = directory.scan(&[
            "--format",
            "gitlab",
            "--output",
            destination.to_str().unwrap(),
        ]);
        assert_eq!(output.status.code(), Some(2));
        assert!(
            std::str::from_utf8(&output.stderr)
                .unwrap()
                .contains("cannot write report")
        );
    }
}

#[test]
fn ci_reports_cannot_overwrite_discovered_sources_or_configuration() {
    let directory = TestDirectory::new();
    let source = directory.write("source.vi", vi_with_separation(false, b"LVIN"));
    let config = directory.write("checks.yaml", "version: 1\n");
    for path in [&source, &config] {
        let original = fs::read(path).unwrap();
        let output = directory.configured_scan(
            &config,
            &["--format", "gitlab", "--output", path.to_str().unwrap()],
        );
        assert_eq!(output.status.code(), Some(2));
        assert!(
            std::str::from_utf8(&output.stderr)
                .unwrap()
                .contains("would overwrite input")
        );
        assert_eq!(fs::read(path).unwrap(), original);
    }
}
