/// Deterministic, bounded, root-contained YAML file discovery and safe file loading.
///
/// Discovery reads regular YAML files in deterministic bytewise path order,
/// excluding `.git/**` and `.codemie/config.yaml`. Directory names such as
/// `codemie` and `codemie-ui` have no runtime exclusion semantics
/// (contracts/cli.md §3).
///
/// Resource budgets (SEC-003, F-003 scope):
/// - Repository file cap: 10,000 files before further processing.
/// - Sidecar per-file bytes cap: 32 MiB (enforced with open-then-fstat on the
///   same file descriptor; descriptor-safe open, not canonicalize-then-reopen).
/// - Aggregate upload bytes cap: 128 MiB tracked across all file parts.
/// - Source file path limit: 4,096 bytes (output rendering, SEC-005).
///
/// Symlinks are rejected unless `--follow-symlinks`; an allowed canonical
/// target must remain within the repository root, and cycles or path-escape
/// attempts fail (contracts/cli.md §3).
use std::collections::HashSet;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::{fs, io};

use crate::cancellation::CancellationToken;
use crate::error::AppError;

// ---------------------------------------------------------------------------
// Resource budget constants (SEC-003)
// ---------------------------------------------------------------------------

/// Maximum number of regular files visited during one discovery pass.
/// Exceeding this cap is a local error before further processing (F-003).
pub const MAX_VISITED_FILES: usize = 10_000;

/// Maximum bytes for a single sidecar file (32 MiB).
pub const MAX_SIDECAR_FILE_BYTES: u64 = 32 * 1024 * 1024;

/// Maximum aggregate bytes across all file parts in one invocation (128 MiB).
pub const MAX_AGGREGATE_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;

/// Maximum bytes for a `source.file` path value in output (SEC-005).
pub const MAX_SOURCE_FILE_PATH_BYTES: usize = 4_096;

// ---------------------------------------------------------------------------
// DiscoveredFile
// ---------------------------------------------------------------------------

/// A discovered YAML file path together with its size in bytes as measured at
/// discovery time via `fs::metadata` (not re-read after the initial stat).
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Absolute canonical path to the YAML file.
    pub path: PathBuf,
    /// File size in bytes as reported by the metadata at discovery time.
    pub byte_len: u64,
}

// ---------------------------------------------------------------------------
// Symlink policy
// ---------------------------------------------------------------------------

/// Walk every prefix of `path` using `symlink_metadata` (does not follow the
/// final symlink).  Returns `Err(AppError::Schema)` as soon as a symlink
/// component is found.
///
/// Only `Component::Normal` components are checked; the root `/`, `.`, and
/// `..` components are skipped (they cannot themselves be symlinks on POSIX).
fn check_no_symlink_in_path(path: &Path) -> Result<(), AppError> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component);
        if !matches!(component, Component::Normal(_)) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(AppError::Schema(format!(
                    "symlink rejected (--follow-symlinks not set): '{}'",
                    current.display()
                )));
            }
            Ok(_) => {}
            Err(e) => {
                return Err(map_io_to_schema(e, &current));
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Path containment
// ---------------------------------------------------------------------------

/// Canonicalize `path` and verify the result is within `canonical_root`.
///
/// Returns the canonical absolute path on success.
/// Returns `AppError::Schema` on path traversal, missing file, or permission
/// denial.
fn check_containment(path: &Path, canonical_root: &Path) -> Result<PathBuf, AppError> {
    let canonical = fs::canonicalize(path).map_err(|e| map_io_to_schema(e, path))?;
    if !canonical.starts_with(canonical_root) {
        return Err(AppError::Schema(format!(
            "path traversal: '{}' resolves outside the repository root",
            path.display()
        )));
    }
    Ok(canonical)
}

// ---------------------------------------------------------------------------
// I/O error mapping
// ---------------------------------------------------------------------------

fn map_io_to_schema(e: io::Error, path: &Path) -> AppError {
    match e.kind() {
        io::ErrorKind::NotFound => {
            AppError::Schema(format!("file not found: '{}'", path.display()))
        }
        io::ErrorKind::PermissionDenied => {
            AppError::Schema(format!("permission denied: '{}'", path.display()))
        }
        _ => AppError::Schema(format!("I/O error for '{}': {e}", path.display())),
    }
}

// ---------------------------------------------------------------------------
// Declaration file loading
// ---------------------------------------------------------------------------

/// Load the declaration file at `file_path`, enforcing:
///
/// 1. Symlink policy: when `follow_symlinks` is `false`, every component of
///    `file_path` is checked with `symlink_metadata`; a symlink at any level
///    produces `AppError::Schema`.
/// 2. Path containment: the canonical path must be within (or equal to)
///    `repo_root`; an escape produces `AppError::Schema`.
/// 3. Readability: permission denial, missing file, and other I/O failures
///    produce `AppError::Schema`.
///
/// Returns the raw file contents as a `String` without parsing YAML (F-004).
pub fn load_declaration_file(
    file_path: &Path,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<String, AppError> {
    // 1. Enforce symlink policy before canonicalize (rejects the path as authored).
    if !follow_symlinks {
        check_no_symlink_in_path(file_path)?;
    }

    // 2. Canonicalize the repo root.
    let canonical_root = fs::canonicalize(repo_root).map_err(|e| {
        AppError::Configuration(format!(
            "cannot canonicalize repository root '{}': {e}",
            repo_root.display()
        ))
    })?;

    // 3. Canonicalize the file path and verify containment.
    let canonical_file = check_containment(file_path, &canonical_root)?;

    // 4. Read via the verified canonical path.
    fs::read_to_string(&canonical_file).map_err(|e| map_io_to_schema(e, file_path))
}

/// Cancellable declaration loading for the invocation coordinator.
///
/// The descriptor is opened once and read in bounded chunks. `max_bytes` is
/// enforced against both fstat and accumulated bytes so a concurrent file
/// growth cannot bypass the pre-allocation limit.
pub fn load_declaration_file_cancellable(
    file_path: &Path,
    repo_root: &Path,
    follow_symlinks: bool,
    max_bytes: u64,
    cancellation: &CancellationToken,
) -> Result<String, AppError> {
    cancellation.checkpoint()?;
    if !follow_symlinks {
        check_no_symlink_in_path(file_path)?;
    }
    let canonical_root = fs::canonicalize(repo_root)
        .map_err(|_| AppError::Configuration("cannot canonicalize repository root".into()))?;
    let canonical_file = check_containment(file_path, &canonical_root)?;
    let bytes = read_file_cancellable(
        &canonical_file,
        max_bytes,
        cancellation,
        "declaration file exceeds its byte limit",
    )?;
    String::from_utf8(bytes)
        .map_err(|_| AppError::YamlParse("declaration is not valid UTF-8".into()))
}

// ---------------------------------------------------------------------------
// Sidecar path resolution
// ---------------------------------------------------------------------------

/// Resolve a sidecar `relative_path` against the declaring YAML's directory.
///
/// The `relative_path` value comes from `contentFrom` in the declaration
/// (cli.md §3: resolves relative to the declaring YAML's directory).
///
/// Validates:
/// - Symlink policy (same rules as [`load_declaration_file`]).
/// - Path containment within `repo_root`.
///
/// Returns the resolved absolute canonical path without reading the file
/// (actual loading and budget enforcement is done by [`load_sidecar_file`]).
pub fn resolve_sidecar_path(
    relative_path: &str,
    declaring_file: &Path,
    repo_root: &Path,
    follow_symlinks: bool,
) -> Result<PathBuf, AppError> {
    let base_dir = declaring_file.parent().ok_or_else(|| {
        AppError::Internal(format!(
            "declaring file '{}' has no parent directory",
            declaring_file.display()
        ))
    })?;

    let resolved = base_dir.join(relative_path);

    if !follow_symlinks {
        check_no_symlink_in_path(&resolved)?;
    }

    let canonical_root = fs::canonicalize(repo_root).map_err(|e| {
        AppError::Configuration(format!(
            "cannot canonicalize repository root '{}': {e}",
            repo_root.display()
        ))
    })?;

    check_containment(&resolved, &canonical_root)
}

// ---------------------------------------------------------------------------
// Sidecar file loading — open-then-fstat (SEC-003)
// ---------------------------------------------------------------------------

/// Load a sidecar file using the open-then-fstat pattern.
///
/// Opens the file once and measures its size via `File::metadata()` on the
/// same open file descriptor.  This avoids the TOCTOU vulnerability of a
/// canonicalize-then-reopen sequence (SEC-003, F-003 acceptance criteria).
///
/// Returns `AppError::Schema` when:
/// - The file exceeds `per_file_budget` bytes.
/// - The file cannot be opened or read (not found, permission denied, etc.).
///
/// The caller is responsible for tracking the aggregate upload budget across
/// multiple sidecar loads (`MAX_AGGREGATE_UPLOAD_BYTES`).
pub fn load_sidecar_file(path: &Path, per_file_budget: u64) -> Result<Vec<u8>, AppError> {
    // Open once — this is the fd we stat and then read from.
    let mut file = fs::File::open(path).map_err(|e| map_io_to_schema(e, path))?;

    // fstat on the open fd (open-then-fstat, SEC-003).
    let byte_len = file
        .metadata()
        .map_err(|e| AppError::Schema(format!("cannot stat '{}': {e}", path.display())))?
        .len();

    if byte_len > per_file_budget {
        return Err(AppError::Schema(format!(
            "sidecar '{}' exceeds the {per_file_budget}-byte per-file limit ({byte_len} bytes)",
            path.display()
        )));
    }

    let mut buf = Vec::with_capacity(byte_len as usize);
    file.read_to_end(&mut buf)
        .map_err(|e| AppError::Schema(format!("error reading '{}': {e}", path.display())))?;

    Ok(buf)
}

/// Cancellable open-then-fstat sidecar read used under R-001's invocation
/// deadline. The cancellation flag is checked between 64 KiB chunks.
pub fn load_sidecar_file_cancellable(
    path: &Path,
    per_file_budget: u64,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, AppError> {
    read_file_cancellable(
        path,
        per_file_budget,
        cancellation,
        "sidecar exceeds its per-file byte limit",
    )
}

fn read_file_cancellable(
    path: &Path,
    max_bytes: u64,
    cancellation: &CancellationToken,
    limit_message: &str,
) -> Result<Vec<u8>, AppError> {
    cancellation.checkpoint()?;
    let mut file = fs::File::open(path).map_err(|e| map_io_to_schema(e, path))?;
    let byte_len = file
        .metadata()
        .map_err(|_| AppError::Schema("cannot stat input file".into()))?
        .len();
    if byte_len > max_bytes {
        return Err(AppError::Schema(limit_message.into()));
    }

    let mut output = Vec::with_capacity(byte_len as usize);
    let mut chunk = [0_u8; 64 * 1024];
    loop {
        cancellation.checkpoint()?;
        let read = file
            .read(&mut chunk)
            .map_err(|e| map_io_to_schema(e, path))?;
        if read == 0 {
            break;
        }
        if output.len().saturating_add(read) as u64 > max_bytes {
            return Err(AppError::Schema(limit_message.into()));
        }
        output.extend_from_slice(&chunk[..read]);
    }
    cancellation.checkpoint()?;
    Ok(output)
}

// ---------------------------------------------------------------------------
// Full repository YAML discovery
// ---------------------------------------------------------------------------

/// Discover all regular YAML files under `root` in deterministic bytewise path
/// order.
///
/// Exclusions:
/// - Any directory entry named `.git` (at any depth).
/// - `.codemie/config.yaml` (the repository configuration file).
///
/// Symlink policy:
/// - `follow_symlinks = false` (default): symlink entries are skipped entirely.
/// - `follow_symlinks = true`: symlinks are followed; their canonical targets
///   must remain within `root`; directory cycles detected via visited-inode
///   tracking do not cause infinite recursion.
///
/// Enforces [`MAX_VISITED_FILES`]; exceeding the cap is a local error before
/// any further processing.
pub fn discover_yaml_files(
    root: &Path,
    follow_symlinks: bool,
) -> Result<Vec<DiscoveredFile>, AppError> {
    let canonical_root = fs::canonicalize(root).map_err(|e| {
        AppError::Configuration(format!(
            "cannot canonicalize repository root '{}': {e}",
            root.display()
        ))
    })?;

    let config_exclusion = canonical_root.join(".codemie").join("config.yaml");

    let mut result: Vec<DiscoveredFile> = Vec::new();
    let mut visited_files: usize = 0;
    let mut visited_dirs: HashSet<PathBuf> = HashSet::new();

    collect_yaml_files(
        &canonical_root,
        &canonical_root,
        &config_exclusion,
        follow_symlinks,
        &mut visited_files,
        &mut visited_dirs,
        &mut result,
    )?;

    // Deterministic bytewise path order (cli.md §3).
    result.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(result)
}

fn collect_yaml_files(
    dir: &Path,
    canonical_root: &Path,
    config_exclusion: &Path,
    follow_symlinks: bool,
    visited_files: &mut usize,
    visited_dirs: &mut HashSet<PathBuf>,
    result: &mut Vec<DiscoveredFile>,
) -> Result<(), AppError> {
    // Track visited canonical directory paths to break symlink cycles.
    let canonical_dir = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_owned());
    if !visited_dirs.insert(canonical_dir) {
        // Already visited; skip to prevent cycle.
        return Ok(());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| AppError::Schema(format!("cannot read directory '{}': {e}", dir.display())))?
        .filter_map(|e| e.ok())
        .collect();

    // Sort entries for deterministic order within each directory.
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();

        // Use symlink_metadata so we can inspect symlinks without following them.
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue, // Transient error — skip this entry.
        };

        let ft = meta.file_type();

        if ft.is_symlink() {
            if !follow_symlinks {
                continue; // Skip symlinks when policy disallows them.
            }
            // Follow the symlink: get the target's metadata.
            let target_meta = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue, // Broken symlink — skip.
            };
            let canonical_target = match fs::canonicalize(&path) {
                Ok(p) => p,
                Err(e) => {
                    return Err(AppError::Schema(format!(
                        "cannot canonicalize symlink target '{}': {e}",
                        path.display()
                    )));
                }
            };
            // Escape check: canonical target must stay within root.
            if !canonical_target.starts_with(canonical_root) {
                return Err(AppError::Schema(format!(
                    "symlink '{}' escapes the repository root",
                    path.display()
                )));
            }
            if target_meta.is_dir() {
                if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                    continue;
                }
                collect_yaml_files(
                    &canonical_target,
                    canonical_root,
                    config_exclusion,
                    follow_symlinks,
                    visited_files,
                    visited_dirs,
                    result,
                )?;
            } else if target_meta.is_file() {
                visit_file(&canonical_target, config_exclusion, visited_files, result)?;
            }
            continue;
        }

        if ft.is_dir() {
            // Skip any directory named `.git` at any depth.
            if path.file_name().map(|n| n == ".git").unwrap_or(false) {
                continue;
            }
            collect_yaml_files(
                &path,
                canonical_root,
                config_exclusion,
                follow_symlinks,
                visited_files,
                visited_dirs,
                result,
            )?;
        } else if ft.is_file() {
            visit_file(&path, config_exclusion, visited_files, result)?;
        }
        // Pipes, sockets, device files: silently skip.
    }

    Ok(())
}

/// Account for a single regular file during traversal and add it to `result`
/// if it is a YAML file that is not the repository config.
fn visit_file(
    path: &Path,
    config_exclusion: &Path,
    visited_files: &mut usize,
    result: &mut Vec<DiscoveredFile>,
) -> Result<(), AppError> {
    // Count every regular file we visit (not just YAML) against the cap.
    *visited_files += 1;
    if *visited_files > MAX_VISITED_FILES {
        return Err(AppError::Schema(format!(
            "repository file cap ({MAX_VISITED_FILES}) exceeded during discovery"
        )));
    }

    // Skip .codemie/config.yaml.
    if path == config_exclusion {
        return Ok(());
    }

    // Include only .yaml and .yml files.
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    if ext != "yaml" && ext != "yml" {
        return Ok(());
    }

    let byte_len = fs::metadata(path).map(|m| m.len()).unwrap_or(0);

    result.push(DiscoveredFile {
        path: path.to_owned(),
        byte_len,
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    // --- Test helpers -------------------------------------------------------

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Create a unique temporary directory and return it plus a guard that
    /// removes it on drop.
    fn temp_dir(label: &str) -> (PathBuf, TempGuard) {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let path = std::env::temp_dir().join(format!("codemie_disc_{pid}_{n}_{label}"));
        fs::create_dir_all(&path).expect("create temp dir");
        let guard = TempGuard(path.clone());
        (path, guard)
    }

    struct TempGuard(PathBuf);

    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Initialise a minimal git repository (just the `.git` directory).
    fn init_git(root: &Path) {
        fs::create_dir_all(root.join(".git")).expect("create .git");
    }

    // --- Constants ----------------------------------------------------------

    #[test]
    fn max_visited_files_is_ten_thousand() {
        assert_eq!(MAX_VISITED_FILES, 10_000);
    }

    #[test]
    fn sidecar_budget_is_32_mib() {
        assert_eq!(MAX_SIDECAR_FILE_BYTES, 32 * 1024 * 1024);
    }

    #[test]
    fn aggregate_budget_is_128_mib() {
        assert_eq!(MAX_AGGREGATE_UPLOAD_BYTES, 128 * 1024 * 1024);
    }

    #[test]
    fn source_path_limit_is_4096() {
        assert_eq!(MAX_SOURCE_FILE_PATH_BYTES, 4_096);
    }

    // --- load_declaration_file — happy path --------------------------------

    #[test]
    fn load_declaration_happy_path() {
        let (root, _g) = temp_dir("happy");
        init_git(&root);
        let decl = root.join("decl.yaml");
        fs::write(&decl, "kind: Assistant\n").unwrap();

        let content =
            load_declaration_file(&decl, &root, false).expect("should load file inside repo root");
        assert_eq!(content, "kind: Assistant\n");
    }

    // --- load_declaration_file — path traversal ----------------------------

    #[test]
    fn load_declaration_path_traversal_rejected() {
        let (root, _g) = temp_dir("traversal");
        init_git(&root);
        // Construct a path that starts inside root but escapes via `..`
        let escape = root
            .join("subdir")
            .join("..")
            .join("..")
            .join("etc")
            .join("passwd");
        let err = load_declaration_file(&escape, &root, false)
            .expect_err("path traversal must be rejected");
        assert_eq!(err.exit_code(), 2, "path traversal must be exit 2");
    }

    // --- load_declaration_file — missing file ------------------------------

    #[test]
    fn load_declaration_missing_file_is_schema_error() {
        let (root, _g) = temp_dir("missing");
        init_git(&root);
        let absent = root.join("does_not_exist.yaml");
        let err = load_declaration_file(&absent, &root, false)
            .expect_err("missing file must produce an error");
        assert_eq!(err.exit_code(), 2);
    }

    // --- load_declaration_file — symlink rejected when follow_symlinks=false

    #[test]
    #[cfg(unix)]
    fn load_declaration_symlink_rejected_when_follow_symlinks_false() {
        let (root, _g) = temp_dir("symrej");
        init_git(&root);
        let target = root.join("real.yaml");
        fs::write(&target, "kind: Skill\n").unwrap();
        let link = root.join("link.yaml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let err = load_declaration_file(&link, &root, false)
            .expect_err("symlink must be rejected when follow_symlinks=false");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("symlink rejected"),
            "error message should mention symlink: {msg}"
        );
    }

    // --- load_declaration_file — symlink allowed when follow_symlinks=true --

    #[test]
    #[cfg(unix)]
    fn load_declaration_symlink_allowed_when_follow_symlinks_true() {
        let (root, _g) = temp_dir("symallow");
        init_git(&root);
        let target = root.join("real.yaml");
        fs::write(&target, "kind: Workflow\n").unwrap();
        let link = root.join("link.yaml");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let content = load_declaration_file(&link, &root, true)
            .expect("symlink to file inside repo must be allowed when follow_symlinks=true");
        assert_eq!(content, "kind: Workflow\n");
    }

    // --- load_declaration_file — symlink escape rejected -------------------

    #[test]
    #[cfg(unix)]
    fn load_declaration_symlink_escape_rejected_when_follow_symlinks_true() {
        let (root, _g) = temp_dir("symescape");
        init_git(&root);
        // Target outside the repo root.
        let outside = std::env::temp_dir().join("outside_target.yaml");
        fs::write(&outside, "secret\n").unwrap();
        let link = root.join("escape.yaml");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        let err = load_declaration_file(&link, &root, true)
            .expect_err("symlink escaping repo must be rejected even with follow_symlinks=true");
        assert_eq!(err.exit_code(), 2);
        let _ = fs::remove_file(&outside);
    }

    // --- load_declaration_file — --repo-root overrides auto-detect ---------

    #[test]
    fn load_declaration_explicit_repo_root_used() {
        // Create two separate trees; supply the deeper one as --repo-root.
        let (outer, _g1) = temp_dir("outer");
        init_git(&outer);
        let inner = outer.join("sub");
        fs::create_dir_all(&inner).unwrap();
        let decl = inner.join("decl.yaml");
        fs::write(&decl, "kind: Datasource\n").unwrap();

        // Using `inner` as repo_root (no .git) should still work because
        // we only need the file to be within the supplied root.
        let content = load_declaration_file(&decl, &inner, false)
            .expect("explicit repo-root that contains the file must work");
        assert_eq!(content, "kind: Datasource\n");
    }

    // --- discover_yaml_files — deterministic order -------------------------

    #[test]
    fn discover_yaml_files_returns_sorted_paths() {
        let (root, _g) = temp_dir("sorted");
        init_git(&root);
        fs::write(root.join("z.yaml"), "").unwrap();
        fs::write(root.join("a.yaml"), "").unwrap();
        fs::write(root.join("m.yaml"), "").unwrap();

        let files = discover_yaml_files(&root, false).unwrap();
        let names: Vec<_> = files
            .iter()
            .map(|f| f.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(
            names, sorted,
            "discover_yaml_files must return paths in sorted order"
        );
    }

    // --- discover_yaml_files — excludes .git -------------------------------

    #[test]
    fn discover_yaml_files_excludes_git_directory() {
        let (root, _g) = temp_dir("gitexcl");
        init_git(&root);
        // Place a YAML inside .git — should be excluded.
        let git_yaml = root.join(".git").join("config.yaml");
        fs::write(&git_yaml, "kind: Assistant\n").unwrap();
        fs::write(root.join("valid.yaml"), "").unwrap();

        let files = discover_yaml_files(&root, false).unwrap();
        assert!(
            files.iter().all(|f| !f.path.starts_with(root.join(".git"))),
            ".git directory must be excluded"
        );
        assert_eq!(files.len(), 1);
    }

    // --- discover_yaml_files — excludes .codemie/config.yaml ---------------

    #[test]
    fn discover_yaml_files_excludes_config_yaml() {
        let (root, _g) = temp_dir("cfgexcl");
        init_git(&root);
        let codemie_dir = root.join(".codemie");
        fs::create_dir_all(&codemie_dir).unwrap();
        fs::write(
            codemie_dir.join("config.yaml"),
            "url: https://x.example.com",
        )
        .unwrap();
        fs::write(root.join("decl.yaml"), "kind: Skill\n").unwrap();

        let files = discover_yaml_files(&root, false).unwrap();
        assert!(
            files
                .iter()
                .all(|f| f.path.file_name() != Some(".codemie/config.yaml".as_ref())),
            ".codemie/config.yaml must be excluded"
        );
        assert!(
            !files
                .iter()
                .any(|f| f.path.ends_with(".codemie/config.yaml")),
            ".codemie/config.yaml must not appear in results"
        );
        assert_eq!(files.len(), 1, "only the declaration should appear");
    }

    // --- discover_yaml_files — codemie/codemie-ui directories not excluded --

    #[test]
    fn discover_yaml_files_includes_codemie_directories() {
        let (root, _g) = temp_dir("cmdir");
        init_git(&root);
        let cm = root.join("codemie");
        let cmui = root.join("codemie-ui");
        fs::create_dir_all(&cm).unwrap();
        fs::create_dir_all(&cmui).unwrap();
        fs::write(cm.join("skill.yaml"), "kind: Skill\n").unwrap();
        fs::write(cmui.join("workflow.yaml"), "kind: Workflow\n").unwrap();

        let files = discover_yaml_files(&root, false).unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        assert!(
            paths.iter().any(|p| p.ends_with("codemie/skill.yaml")),
            "codemie/ directory must not be excluded"
        );
        assert!(
            paths
                .iter()
                .any(|p| p.ends_with("codemie-ui/workflow.yaml")),
            "codemie-ui/ directory must not be excluded"
        );
    }

    // --- discover_yaml_files — visited-files cap ---------------------------

    #[test]
    fn discover_yaml_files_enforces_visited_cap() {
        let (root, _g) = temp_dir("cap");
        init_git(&root);
        // Write MAX_VISITED_FILES + 1 regular files (non-YAML so discovery
        // counts them without adding to results, making the test fast).
        for i in 0..=MAX_VISITED_FILES {
            fs::write(root.join(format!("file{i}.txt")), "").unwrap();
        }
        let err =
            discover_yaml_files(&root, false).expect_err("cap exceeded must produce an error");
        assert_eq!(err.exit_code(), 2);
    }

    // --- discover_yaml_files — nested sidecar example ----------------------

    #[test]
    fn discover_yaml_files_nested_sidecar_example() {
        let (root, _g) = temp_dir("nested");
        init_git(&root);
        let skills_dir = root.join("skills").join("my-skill");
        fs::create_dir_all(&skills_dir).unwrap();
        fs::write(skills_dir.join("skill.yaml"), "kind: Skill\n").unwrap();
        fs::write(skills_dir.join("content.md"), "# content").unwrap();

        let files = discover_yaml_files(&root, false).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].path.ends_with("skill.yaml"));
    }

    // --- resolve_sidecar_path ----------------------------------------------

    #[test]
    fn resolve_sidecar_path_happy() {
        let (root, _g) = temp_dir("sidecar_happy");
        init_git(&root);
        let decl = root.join("skill.yaml");
        fs::write(&decl, "").unwrap();
        let content = root.join("content.md");
        fs::write(&content, "# hello").unwrap();

        let resolved = resolve_sidecar_path("content.md", &decl, &root, false)
            .expect("co-located sidecar must resolve");
        assert!(resolved.ends_with("content.md"));
    }

    #[test]
    fn resolve_sidecar_path_traversal_rejected() {
        let (root, _g) = temp_dir("sidecar_trav");
        init_git(&root);
        let decl = root.join("skill.yaml");
        fs::write(&decl, "").unwrap();

        let err = resolve_sidecar_path("../../etc/passwd", &decl, &root, false)
            .expect_err("traversal via sidecar path must be rejected");
        assert_eq!(err.exit_code(), 2);
    }

    // --- load_sidecar_file — open-then-fstat budget ------------------------

    #[test]
    fn load_sidecar_file_within_budget_succeeds() {
        let (root, _g) = temp_dir("sidecar_ok");
        let path = root.join("content.md");
        fs::write(&path, b"hello world" as &[u8]).unwrap();

        let bytes =
            load_sidecar_file(&path, MAX_SIDECAR_FILE_BYTES).expect("file within budget must load");
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn load_sidecar_file_exceeds_budget_rejected() {
        let (root, _g) = temp_dir("sidecar_big");
        let path = root.join("big.md");
        // Write 5 bytes but set the budget to 4 bytes.
        fs::write(&path, b"hello" as &[u8]).unwrap();

        let err = load_sidecar_file(&path, 4).expect_err("file exceeding budget must be rejected");
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn load_sidecar_file_missing_is_error() {
        let (root, _g) = temp_dir("sidecar_miss");
        let absent = root.join("no_such_file.md");

        let err = load_sidecar_file(&absent, MAX_SIDECAR_FILE_BYTES)
            .expect_err("missing sidecar must produce an error");
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn cancellable_sidecar_read_observes_invocation_cancellation() {
        let (root, _g) = temp_dir("sidecar_cancelled");
        let path = root.join("content.md");
        fs::write(&path, b"content" as &[u8]).unwrap();
        let cancellation = CancellationToken::default();
        cancellation.cancel();

        let error = load_sidecar_file_cancellable(&path, MAX_SIDECAR_FILE_BYTES, &cancellation)
            .expect_err("cancelled sidecar reading must stop before opening the file");
        assert!(matches!(error, AppError::Timeout(_)));
    }

    // --- DiscoveredFile struct ----------------------------------------------

    #[test]
    fn discovered_file_stores_path_and_size() {
        let f = DiscoveredFile {
            path: PathBuf::from("/tmp/decl.yaml"),
            byte_len: 1024,
        };
        assert_eq!(f.byte_len, 1024);
        assert_eq!(f.path, PathBuf::from("/tmp/decl.yaml"));
    }
}
