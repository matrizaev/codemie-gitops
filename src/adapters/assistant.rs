/// Assistant entity adapter stub.
///
/// Uses exact `(project, slug)` resolution via the direct API lookup.
/// Absent identity → POST/created; present identity → PUT/updated on every
/// invocation (ADR-002; A-001 scope).
use crate::error::AppError;

/// Assistant adapter for identity resolution and write-through CRUD.
#[derive(Debug)]
pub struct AssistantAdapter;

impl AssistantAdapter {
    pub fn new() -> Self {
        AssistantAdapter
    }

    /// Apply an Assistant declaration to the target API.
    ///
    /// Full implementation is in A-001.
    pub async fn apply(&self, _project: &str, _slug: &str) -> Result<(), AppError> {
        todo!("Assistant adapter implemented in A-001")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_adapter_constructs() {
        let _adapter = AssistantAdapter::new();
    }
}
