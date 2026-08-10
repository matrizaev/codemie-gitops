/// Workflow entity adapter stub.
///
/// Uses the reserved `meta_config` identity and explicit by-UUID legacy
/// adoption (ADR-008). Display name never selects or vetoes the explicit
/// candidate. Unconditional existing-entity PUT on every invocation.
/// String `meta_config` strict decode/canonical encode/preservation merge
/// is required (W-001/W-002 scope).
use crate::error::AppError;

/// Workflow adapter for meta_config identity resolution and write-through.
#[derive(Debug)]
pub struct WorkflowAdapter;

impl WorkflowAdapter {
    pub fn new() -> Self {
        WorkflowAdapter
    }

    /// Apply a Workflow declaration to the target API.
    ///
    /// Full implementation including explicit adoption and meta_config merge
    /// is in W-001/W-002.
    pub async fn apply(
        &self,
        _project: &str,
        _slug: &str,
        _adopt_workflow_id: Option<&str>,
    ) -> Result<(), AppError> {
        todo!("Workflow adapter implemented in W-001/W-002")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_adapter_constructs() {
        let _adapter = WorkflowAdapter::new();
    }
}
