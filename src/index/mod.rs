/// Repository index: offline natural-key and graph-local reference closure.
///
/// The index is built from discovered and parsed declarations to support
/// duplicate detection, cross-file reference validation, and Workflow-local
/// actor reference checks. No network access occurs during index construction
/// or querying (contracts/cli.md §4).
use std::path::PathBuf;

use crate::error::AppError;

/// The entity kind, used as part of the natural key in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Assistant,
    Workflow,
    Skill,
    Datasource,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EntityKind::Assistant => "Assistant",
            EntityKind::Workflow => "Workflow",
            EntityKind::Skill => "Skill",
            EntityKind::Datasource => "Datasource",
        }
    }
}

/// A declaration entry in the repository index, keyed by
/// `(project, kind, slug_or_name)`.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    /// The effective project for this declaration.
    pub project: String,
    /// The entity kind.
    pub kind: EntityKind,
    /// The kind's natural-key field value (slug for Assistant/Workflow,
    /// name for Skill/Datasource).
    pub key: String,
    /// Source file path, used in diagnostics (bounded to 4,096 bytes in output).
    pub source_path: PathBuf,
}

/// The built repository index for one invocation.
#[derive(Debug, Default)]
pub struct RepositoryIndex {
    entries: Vec<IndexEntry>,
}

impl RepositoryIndex {
    /// Build a repository index from a set of index entries.
    pub fn new(entries: Vec<IndexEntry>) -> Self {
        RepositoryIndex { entries }
    }

    /// Return all entries in the index.
    pub fn entries(&self) -> &[IndexEntry] {
        &self.entries
    }

    /// Look up entries by project and kind.
    pub fn find_by_project_kind(
        &self,
        project: &str,
        kind: EntityKind,
    ) -> Vec<&IndexEntry> {
        self.entries
            .iter()
            .filter(|e| e.project == project && e.kind == kind)
            .collect()
    }
}

/// Build the repository index from parsed declarations.
///
/// This is a stub; full index construction including duplicate detection and
/// graph-local reference closure is implemented in F-005.
pub fn build_index(_parsed_declarations: &[()]) -> Result<RepositoryIndex, AppError> {
    todo!("repository index construction implemented in F-005")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_kind_as_str() {
        assert_eq!(EntityKind::Assistant.as_str(), "Assistant");
        assert_eq!(EntityKind::Workflow.as_str(), "Workflow");
        assert_eq!(EntityKind::Skill.as_str(), "Skill");
        assert_eq!(EntityKind::Datasource.as_str(), "Datasource");
    }

    #[test]
    fn empty_index_returns_empty_entries() {
        let idx = RepositoryIndex::new(vec![]);
        assert!(idx.entries().is_empty());
    }

    #[test]
    fn find_by_project_kind_filters_correctly() {
        let entries = vec![
            IndexEntry {
                project: "proj-a".into(),
                kind: EntityKind::Assistant,
                key: "slug-1".into(),
                source_path: PathBuf::from("a.yaml"),
            },
            IndexEntry {
                project: "proj-a".into(),
                kind: EntityKind::Skill,
                key: "skill-1".into(),
                source_path: PathBuf::from("b.yaml"),
            },
            IndexEntry {
                project: "proj-b".into(),
                kind: EntityKind::Assistant,
                key: "slug-2".into(),
                source_path: PathBuf::from("c.yaml"),
            },
        ];
        let idx = RepositoryIndex::new(entries);
        let found = idx.find_by_project_kind("proj-a", EntityKind::Assistant);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].key, "slug-1");
    }
}
