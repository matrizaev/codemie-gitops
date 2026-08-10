/// Operation request projection stub.
///
/// Produces typed `Create` or `Update` plans from the authored declaration
/// intent and the selected identity result. Rules (ADR-002, contracts/cli.md §5):
///
/// - Omitted and explicit-null optional-null fields → present JSON null in each
///   applicable JSON POST/PUT.
/// - Authoring-only, operation-inapplicable, read-only, and tool/mixed-owned
///   members receive no fabricated null.
/// - No default filling; no equality-based write suppression.
/// - Create-only fields are absent from Update.
///
/// Full projection including Workflow `meta_config` merge and Datasource peer
/// kind projections is implemented in F-006.
use serde_json::Value;

use crate::error::AppError;

/// A typed operation plan produced by the projector.
///
/// The projector never selects between Create and Update based on field state
/// — that determination is made by the identity resolver (absent → Create,
/// present → Update).
#[derive(Debug)]
pub enum OperationPlan {
    /// No existing identity found: issue a POST.
    Create { request_body: Value },
    /// Existing identity found: issue a PUT.
    Update { server_id: String, request_body: Value },
}

/// Project a parsed declaration into an operation plan.
///
/// This is a stub; full projection with presence/null classification and
/// Workflow/Datasource transforms is implemented in F-006.
pub fn project_declaration(
    _declaration: &Value,
    _server_id: Option<String>,
) -> Result<OperationPlan, AppError> {
    todo!("request projection implemented in F-006")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_plan_create_variant() {
        let plan = OperationPlan::Create { request_body: serde_json::json!({}) };
        assert!(matches!(plan, OperationPlan::Create { .. }));
    }

    #[test]
    fn operation_plan_update_variant() {
        let plan = OperationPlan::Update {
            server_id: "some-uuid".into(),
            request_body: serde_json::json!({}),
        };
        assert!(matches!(plan, OperationPlan::Update { .. }));
    }
}
