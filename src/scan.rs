//! Recursive discovery of supported LabVIEW files.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const SUPPORTED_EXTENSIONS: &[&str] = &["vi", "vit", "vim", "ctl", "lvproj", "lvclass", "lvlib"];

/// Finds supported LabVIEW file candidates under `root`.
///
/// Traversal is recursive, skips symbolic links and excluded child directories,
/// and returns paths relative to `root` in sorted order. Exclusions match literal
/// folder names case-sensitively at every depth; the root itself is never excluded.
///
/// # Errors
///
/// Returns an error if `root` is not a directory or any filesystem operation
/// fails during traversal.
pub fn discover_files(root: &Path, exclude_folders: &[String]) -> io::Result<Vec<PathBuf>> {
    let metadata = fs::metadata(root)
        .map_err(|error| with_path_context("cannot inspect scan folder", root, error))?;
    if !metadata.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("scan path is not a directory: {}", root.display()),
        ));
    }

    let excluded: HashSet<&OsStr> = exclude_folders.iter().map(OsStr::new).collect();
    let mut directories = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(directory) = directories.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| with_path_context("cannot read directory", &directory, error))?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                io::Error::new(
                    error.kind(),
                    format!("cannot read an entry in '{}': {error}", directory.display()),
                )
            })?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| with_path_context("cannot inspect", &path, error))?;

            if file_type.is_symlink() {
                continue;
            }

            if file_type.is_dir() {
                if !excluded.contains(entry.file_name().as_os_str()) {
                    directories.push(path);
                }
            } else if file_type.is_file() && has_supported_extension(&path) {
                let relative_path = path.strip_prefix(root).map_err(|error| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!(
                            "discovered path '{}' is outside scan folder '{}': {error}",
                            path.display(),
                            root.display()
                        ),
                    )
                })?;
                files.push(relative_path.to_path_buf());
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Returns whether `path` has a supported LabVIEW extension, ignoring case.
fn has_supported_extension(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| {
            SUPPORTED_EXTENSIONS
                .iter()
                .any(|supported| extension.eq_ignore_ascii_case(supported))
        })
}

/// Adds the operation and path to an I/O error while preserving its kind.
fn with_path_context(action: &str, path: &Path, error: io::Error) -> io::Error {
    io::Error::new(
        error.kind(),
        format!("{action} '{}': {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::{discover_files, has_supported_extension};
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> io::Result<Self> {
            loop {
                let id = NEXT_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir()
                    .join(format!("wirelint-scan-test-{}-{id}", std::process::id()));
                match fs::create_dir(&path) {
                    Ok(()) => return Ok(Self(path)),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => return Err(error),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn recognizes_supported_extensions_without_case_sensitivity() {
        for extension in ["vi", "vit", "vim", "ctl", "lvproj", "lvclass", "lvlib"] {
            assert!(has_supported_extension(Path::new(&format!(
                "example.{extension}"
            ))));
            assert!(has_supported_extension(Path::new(&format!(
                "example.{}",
                extension.to_ascii_uppercase()
            ))));
        }

        assert!(!has_supported_extension(Path::new("example.llb")));
        assert!(!has_supported_extension(Path::new("readme.txt")));
    }

    #[test]
    fn discovers_supported_files_recursively_in_sorted_order() {
        let root = TestDirectory::new().unwrap();
        let nested = root.path().join("nested");
        fs::create_dir(&nested).unwrap();

        for path in [
            root.path().join("a.vi"),
            root.path().join("b.LVPROJ"),
            nested.join("c.ctl"),
            nested.join("d.lvclass"),
            nested.join("e.lvlib"),
            nested.join("f.vim"),
            nested.join("g.vit"),
            nested.join("ignored.llb"),
            nested.join("ignored.txt"),
        ] {
            fs::write(path, []).unwrap();
        }

        let files = discover_files(root.path(), &[]).unwrap();

        assert_eq!(
            files,
            vec![
                PathBuf::from("a.vi"),
                PathBuf::from("b.LVPROJ"),
                PathBuf::from("nested").join("c.ctl"),
                PathBuf::from("nested").join("d.lvclass"),
                PathBuf::from("nested").join("e.lvlib"),
                PathBuf::from("nested").join("f.vim"),
                PathBuf::from("nested").join("g.vit"),
            ]
        );
    }

    #[test]
    fn rejects_a_file_as_scan_root() {
        let root = TestDirectory::new().unwrap();
        let file = root.path().join("file.vi");
        fs::write(&file, []).unwrap();

        let error = discover_files(&file, &[]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert!(error.to_string().contains("scan path is not a directory"));
    }

    #[cfg(unix)]
    #[test]
    fn skips_symbolic_links_inside_the_scan_root() {
        use std::os::unix::fs::symlink;

        let root = TestDirectory::new().unwrap();
        let target = TestDirectory::new().unwrap();
        let target_file = target.path().join("external.vi");
        fs::write(&target_file, []).unwrap();
        symlink(target.path(), root.path().join("linked-directory")).unwrap();
        symlink(&target_file, root.path().join("linked.vi")).unwrap();

        let files = discover_files(root.path(), &[]).unwrap();

        assert!(files.is_empty());
    }

    #[test]
    fn prunes_excluded_folder_names_at_every_depth_without_matching_prefixes() {
        let root = TestDirectory::new().unwrap();
        let nested = root.path().join("nested");
        fs::create_dir(&nested).unwrap();
        for parent in [root.path(), nested.as_path()] {
            for name in [".git", ".lv-venv", ".git-backup", ".lv-venv-backup"] {
                let directory = parent.join(name).join("deeper");
                fs::create_dir_all(&directory).unwrap();
                fs::write(directory.join("file.vi"), []).unwrap();
            }
        }
        let exclusions = vec![".git".to_owned(), ".lv-venv".to_owned()];
        let files = discover_files(root.path(), &exclusions).unwrap();
        let mut expected = Vec::new();
        for prefix in [Path::new(""), Path::new("nested")] {
            for name in [".git-backup", ".lv-venv-backup"] {
                expected.push(prefix.join(name).join("deeper").join("file.vi"));
            }
        }
        expected.sort();
        assert_eq!(files, expected);
        assert_eq!(discover_files(root.path(), &[]).unwrap().len(), 8);
    }

    #[test]
    fn exclusions_are_case_sensitive_and_apply_only_to_child_directories() {
        let root = TestDirectory::new().unwrap();
        let excluded_root = root.path().join(".git");
        fs::create_dir(&excluded_root).unwrap();
        fs::write(excluded_root.join("root.vi"), []).unwrap();
        fs::create_dir(excluded_root.join(".GIT")).unwrap();
        fs::write(excluded_root.join(".GIT").join("upper.vi"), []).unwrap();
        fs::write(excluded_root.join("excluded.vi"), []).unwrap();

        assert_eq!(
            discover_files(
                &excluded_root,
                &[".git".to_owned(), "excluded.vi".to_owned()]
            )
            .unwrap(),
            vec![
                PathBuf::from(".GIT").join("upper.vi"),
                PathBuf::from("excluded.vi"),
                PathBuf::from("root.vi"),
            ]
        );
    }
}
