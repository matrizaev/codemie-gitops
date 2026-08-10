/// Deterministic, bounded, root-contained YAML file discovery.
///
/// Discovery reads regular YAML files in deterministic bytewise path order,
/// excluding `.git/**` and `.codemie/config.yaml`. Directory names such as
/// `codemie` and `codemie-ui` have no runtime exclusion semantics
/// (contracts/cli.md §3).
///
/// Resource budgets (SEC-003, F-003 scope):
/// - Repository file cap: 10,000 files before further processing.
/// - Sidecar per-file bytes cap: 32 MiB (enforced in F-003 using
///   open-then-fstat on the same fd; descriptor-safe open, not
///   canonicalize-then-reopen).
/// - Aggregate upload bytes cap: 128 MiB tracked across all file parts.
///
/// Symlinks are rejected unless `--follow-symlinks`; an allowed canonical
/// target must remain within the repository root, and cycles or path-escape
/// attempts fail.
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// A discovered YAML file path together with its size in bytes as measured
/// at discovery time (not re-read after initial open).
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Absolute path to the YAML file.
    pub path: PathBuf,
    /// File size in bytes as reported by the metadata at discovery time.
    pub byte_len: u64,
}

/// Maximum number of repository files visited in one discovery pass.
pub const MAX_VISITED_FILES: usize = 10_000;

/// Discover all regular YAML files under `root` in deterministic bytewise path
/// order.
///
/// This is a stub; full safe traversal (symlink containment, cycle detection,
/// size bounds) is implemented in F-003.
pub fn discover_yaml_files(
    root: &Path,
    _follow_symlinks: bool,
) -> Result<Vec<DiscoveredFile>, AppError> {
    let _ = root;
    todo!("file discovery implemented in F-003")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_visited_files_constant() {
        // Verify the resource budget constant is set to the spec value.
        assert_eq!(MAX_VISITED_FILES, 10_000);
    }

    #[test]
    fn discovered_file_stores_path_and_size() {
        let f = DiscoveredFile { path: PathBuf::from("/tmp/decl.yaml"), byte_len: 1024 };
        assert_eq!(f.byte_len, 1024);
        assert_eq!(f.path, PathBuf::from("/tmp/decl.yaml"));
    }
}
