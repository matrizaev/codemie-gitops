/// Assistant entity adapter — A-001.
///
/// Exact `(project, slug)` resolution via `GET /v1/assistants/slug/{slug}?project={project}`.
/// Absent identity → POST/created; present identity → unconditional PUT/updated.
/// Assistant intentionally has no `/v1/user` admin preflight. Its direct lookup
/// and existing-row write ability are sealed with the projected request.
///
/// ## Source traceability
///
/// - Requirements: FR-005/006/015/021, DR-003/005
/// - Architecture: ADR-002
/// - Task: A-001
/// - Manifest: §entities.Assistant routes.resolve / create / update
use serde::Deserialize;

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value, preflight_visibility};
use crate::parse::ParsedDeclaration;
use crate::projection::{AssistantReferenceMap, ExistingEntity, project_with_assistant_references};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence,
    decode_write_response, prove_write,
};

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

/// Fields consumed from the direct Assistant identity response.
#[derive(Deserialize)]
struct AssistantLookupResponse {
    id: String,
    user_abilities: Vec<String>,
}

/// Minimal fields consumed from a successful modifying response.
#[derive(Deserialize)]
struct AssistantWriteResponse {
    #[serde(rename = "assistantId", alias = "id")]
    id: Option<String>,
}

/// Actual strict direct-lookup evidence required by the Assistant seal.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    _slug: String,
    target: ResolutionTarget,
    _write_ability: Option<WriteAbilityEvidence>,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
    }

    pub(super) fn target(&self) -> &ResolutionTarget {
        &self.target
    }
}

// ---------------------------------------------------------------------------
// Public adapter function
// ---------------------------------------------------------------------------

/// Apply an Assistant declaration to the target API.
///
/// 1. Calls `GET /v1/assistants/slug/{slug}?project={project}` to resolve identity.
/// 2. 404 → Create path; 200 → Update path.
/// 3. Projects the declaration into a `WritePlan`.
/// 4. Dispatches `POST /v1/assistants` (create) or `PUT /v1/assistants/{id}` (update).
///
pub async fn apply(
    client: &ApiClient,
    _base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
    project_name: &str,
    slug: &str,
) -> Result<ApplyResult, AppError> {
    let visibility = preflight_visibility(client, project_name).await?;
    // Step 1: Resolve identity.
    let resolve_path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project_name)
    );
    let existing: Option<AssistantLookupResponse> = client.get_optional(&resolve_path).await?;

    let resolution = match &existing {
        Some(existing) => CompletedResolution {
            effective_project: project_name.to_owned(),
            _slug: slug.to_owned(),
            target: ResolutionTarget::Update {
                server_id: existing.id.clone(),
            },
            _write_ability: Some(prove_write(&existing.user_abilities, "Assistant")?),
        },
        None => CompletedResolution {
            effective_project: project_name.to_owned(),
            _slug: slug.to_owned(),
            target: ResolutionTarget::Create,
            _write_ability: None,
        },
    };

    let existing_entity = existing.as_ref().map(|e| ExistingEntity {
        server_id: e.id.clone(),
        meta_config: None,
    });

    // Step 2: Project.
    let references = resolve_authored_references(client, _base_url, decl).await?;
    let plan = project_with_assistant_references(decl, existing_entity.as_ref(), &references)?;

    // Step 3: seal the completed direct-lookup evidence with projection. The
    // modifying dispatcher accepts no raw or partial-evidence `WritePlan`.
    let prepared = PreparedWrite::assistant(client, visibility, resolution, plan)?;
    dispatch(prepared).await
}

async fn resolve_authored_references(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    declaration: &ParsedDeclaration,
) -> Result<AssistantReferenceMap, AppError> {
    let value = declaration.reference_value()?;
    let spec = value
        .get("spec")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| AppError::Internal("validated Assistant omitted spec".into()))?;
    let mut references = AssistantReferenceMap::default();

    for reference in required_array(spec, "sub_assistants")? {
        let (project, slug) = natural_pair(reference, "slug")?;
        references
            .assistant_ids
            .push(resolve_reference(client, base_url, project, slug).await?);
    }
    for reference in required_array(spec, "skills")? {
        let (project, name) = natural_pair(reference, "name")?;
        references
            .skill_ids
            .push(super::skill::resolve_reference(client, base_url, project, name).await?);
    }
    for context in required_array(spec, "context")? {
        let context = context.as_object().ok_or_else(|| {
            AppError::Internal("validated Assistant context item was not an object".into())
        })?;
        let context_type = context
            .get("context_type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| AppError::Internal("validated Assistant context omitted type".into()))?;
        let reference = context.get("ref").ok_or_else(|| {
            AppError::Internal("validated Assistant context omitted reference".into())
        })?;
        let (project, repo_name) = natural_pair(reference, "repo_name")?;
        super::datasource::resolve_reference(client, base_url, project, repo_name).await?;
        references.context.push(serde_json::json!({
            "context_type": context_type,
            "name": repo_name,
        }));
    }
    Ok(references)
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
) -> Result<&'a [serde_json::Value], AppError> {
    object
        .get(field)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| AppError::Internal("validated Assistant reference array is missing".into()))
}

fn natural_pair<'a>(
    value: &'a serde_json::Value,
    selector: &str,
) -> Result<(&'a str, &'a str), AppError> {
    let object = value.as_object().ok_or_else(|| {
        AppError::Internal("validated Assistant reference was not an object".into())
    })?;
    let project = object
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Internal("validated Assistant reference omitted project".into())
        })?;
    let selector = object
        .get(selector)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Internal("validated Assistant reference omitted selector".into())
        })?;
    Ok((project, selector))
}

async fn dispatch(prepared: PreparedWrite<'_>) -> Result<ApplyResult, AppError> {
    let (action, resolved_update_id) = match prepared.target() {
        ResolutionTarget::Create => (ApplyAction::Created, None),
        ResolutionTarget::Update { server_id } => (ApplyAction::Updated, Some(server_id.clone())),
    };
    let response = ApiClient::dispatch_prepared(prepared).await?;
    let response: AssistantWriteResponse = decode_write_response(response)?.ok_or_else(|| {
        AppError::Internal("Assistant modifying request cannot return a conflict signal".into())
    })?;
    let server_id = match (response.id, resolved_update_id) {
        (Some(id), _) => id,
        (None, Some(id)) => id,
        (None, None) => {
            return Err(AppError::Internal(
                "Assistant update path missing resolved server identity".into(),
            ));
        }
    };
    ApplyResult::from_server_response(action, server_id)
}

/// Resolve an Assistant natural reference without writing it (DR-003/W-002).
pub async fn resolve_reference(
    client: &ApiClient,
    _base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
) -> Result<String, AppError> {
    let path = format!(
        "/v1/assistants/slug/{}?project={}",
        encode_query_value(slug),
        encode_query_value(project_name)
    );
    client
        .get_optional::<AssistantLookupResponse>(&path)
        .await?
        .map(|item| item.id)
        .ok_or_else(|| {
            AppError::Reconciliation("referenced Assistant is missing on the target server".into())
        })
}

/// Verify that the authored identity resolves to the route ID returned by the
/// modifying request. This is a read-only post-write check (R-001).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let actual = resolve_reference(client, base_url, project_name, slug).await?;
    if actual == expected_server_id {
        Ok(())
    } else {
        Err(AppError::Reconciliation(
            "Assistant write may have committed but identity verification did not match".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests (A-001)
// ---------------------------------------------------------------------------
