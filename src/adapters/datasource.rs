/// Datasource entity adapter stub.
///
/// One exhaustive resolver and peer per-kind create/update projections.
/// Ordinary write-through CRUD; no dedicated lifecycle operation.
/// Visibility preflight (`GET /v1/user`) required before any write
/// per ADR-012 Option A (D-001 scope).
///
/// File Datasource multipart basename safety enforced before upload:
/// basenames containing C0/C1 controls, CR, LF, NUL, or path separator
/// characters are rejected (SEC-005; D-001 scope).
use crate::error::AppError;

/// The Datasource kind, distinguishing projection and transport paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatasourceKind {
    Url,
    File,
    S3Compatible,
    AzureBlob,
    GcsBucket,
    Confluence,
    Jira,
    Github,
    Gitlab,
    Bitbucket,
}

/// Datasource adapter for exhaustive identity resolution and write-through CRUD.
#[derive(Debug)]
pub struct DatasourceAdapter;

impl DatasourceAdapter {
    pub fn new() -> Self {
        DatasourceAdapter
    }

    /// Apply a Datasource declaration of the given kind.
    ///
    /// Full peer-kind mapping, visibility preflight, and pagination cap
    /// (1,000 pages / 100,000 items) are implemented in D-001.
    pub async fn apply(
        &self,
        _kind: DatasourceKind,
        _project: &str,
        _name: &str,
    ) -> Result<(), AppError> {
        todo!("Datasource adapter implemented in D-001")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn datasource_adapter_constructs() {
        let _adapter = DatasourceAdapter::new();
    }

    #[test]
    fn datasource_kind_variants_are_distinct() {
        assert_ne!(DatasourceKind::Url, DatasourceKind::File);
        assert_ne!(DatasourceKind::S3Compatible, DatasourceKind::AzureBlob);
    }
}
