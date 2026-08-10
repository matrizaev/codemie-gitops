/// Offline natural and graph-local reference validation.
///
/// This module validates that all cross-entity references in the repository
/// index resolve to exactly one declaration of the correct kind and project,
/// and that Workflow-local actor references are consistent within the
/// Workflow's own declaration. No network access occurs (contracts/cli.md §4).
///
/// Full reference closure including duplicate/missing/wrong-kind/cross-project
/// checks is implemented in F-005.
use crate::{error::AppError, index::RepositoryIndex};

/// Validate all natural and graph-local references across the repository index.
///
/// This is a stub; full deterministic reference closure is implemented in F-005.
pub fn validate_references(_index: &RepositoryIndex) -> Result<(), AppError> {
    todo!("reference validation implemented in F-005")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::RepositoryIndex;

    #[test]
    fn reference_module_compiles() {
        // Structural test: verify the module and its public API are wired.
        let _: fn(&RepositoryIndex) -> Result<(), AppError> = validate_references;
    }
}
