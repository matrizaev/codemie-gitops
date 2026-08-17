/// Workflow entity adapter — W-001.
///
/// Identity resolved from the reserved workflow-identity meta_config marker.
/// Enumeration, marker classification, and the shared page scan live in the
/// enumeration submodule; this module owns apply/adoption, verification,
/// snapshots, and reference resolution.
use std::collections::BTreeSet;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::declaration_schema::ExecutionConfigAssistantsItem;
use crate::error::AppError;
use crate::http::{ApiClient, encode_path_segment, encode_query_value, preflight_visibility};

use crate::parse::{ParsedDeclaration, ParsedDeclarationRef};
use crate::projection::{
    ExistingEntity, RequestBody, WorkflowReferenceMap, WritePlan, project_with_workflow_references,
};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence, assistant,
    datasource, decode_write_response, prove_write, skill,
};

mod enumeration;

use enumeration::{
    IDENTITY_KEY, MarkerClassification, PassEvidence, WorkflowItem, classify_marker, enumerate_all,
};

/// Minimal contract required by the shared Workflow detail gateway. Save owns
/// its richer DTO while this adapter owns selection and identity policy.
pub(crate) trait WorkflowSnapshotContract: DeserializeOwned {
    fn id(&self) -> Option<&str>;
    fn project(&self) -> Option<&str>;
    fn meta_config(&self) -> Option<&str>;
    fn mode(&self) -> Option<&str>;
    fn deprecated(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(untagged)]
enum WorkflowWriteResponse {
    Direct { id: String },
    Nested { data: WorkflowWriteIdentity },
}

#[derive(Deserialize)]
struct WorkflowWriteIdentity {
    id: String,
}

impl WorkflowWriteResponse {
    fn into_id(self) -> String {
        match self {
            Self::Direct { id } => id,
            Self::Nested { data } => data.id,
        }
    }
}

/// `meta_config` is nullable in the pinned API but its response member is
/// required. A newtype preserves the difference between explicit `null` and
/// an omitted field during Serde struct decoding.
struct ResolvedWorkflowReferences {
    map: WorkflowReferenceMap,
}

/// Concrete Workflow resolution evidence owned by `PreparedWrite`.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    _slug: String,
    target: ResolutionTarget,
    _scope_scans: Vec<PassEvidence>,
    _resolved_references: WorkflowReferenceMap,
    _detail_id: Option<String>,
    write_abilities: Vec<WriteAbilityEvidence>,
    _adoption: bool,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
    }

    pub(super) fn target(&self) -> &ResolutionTarget {
        &self.target
    }

    pub(super) fn write_ability_is_proven(&self) -> bool {
        !self.write_abilities.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Public adapter entry point
// ---------------------------------------------------------------------------

pub struct ApplyRequest<'a> {
    pub declaration: &'a ParsedDeclaration,
    pub project_name: &'a str,
    pub slug: &'a str,
    pub adopt_workflow_id: Option<&'a str>,
}

pub async fn apply(client: &ApiClient, request: ApplyRequest<'_>) -> Result<ApplyResult, AppError> {
    let initial_visibility = preflight_visibility(client, request.project_name).await?;
    let creator_user_id = initial_visibility.authenticated_user_id().to_owned();
    let authored_display_name = request
        .declaration
        .workflow_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::Schema("workflow: spec.name is required".into()))?;
    // Both scopes are always exhausted before either ordinary selection or
    // explicit adoption. This proves the zero/exact/ambiguous marker state and
    // fails closed on invalid target-project metadata.
    let scan = enumerate_all(
        client,
        request.project_name,
        request.slug,
        &creator_user_id,
        Some(authored_display_name),
    )
    .await?;

    let (existing_entity, target, detail_id, write_abilities, adoption) = if let Some(adopt_id) =
        request.adopt_workflow_id
    {
        if !scan.exact_matches.is_empty() {
            return Err(AppError::AdoptionRequired(
                "workflow adoption requires zero existing exact identity markers".into(),
            ));
        }
        let detail = fetch_detail(client, adopt_id).await?;
        if detail.id != adopt_id
            || detail.project != request.project_name
            || detail.created_by.id != creator_user_id
        {
            return Err(AppError::ResolutionUnstable(
                "workflow adoption candidate does not match the selected project and id".into(),
            ));
        }
        if classify_marker(
            detail.meta_config.0.as_deref(),
            &detail.project,
            request.project_name,
            request.slug,
            &creator_user_id,
        ) != MarkerClassification::Unmarked
        {
            return Err(AppError::IdentityMarkerInvalid(
                "workflow adoption candidate has a reserved or invalid identity marker".into(),
            ));
        }
        let ability = prove_write(&detail.user_abilities, "Workflow")?;
        let id = detail.id;
        (
            Some(ExistingEntity {
                server_id: id.clone(),
                meta_config: detail.meta_config.0,
            }),
            ResolutionTarget::Update {
                server_id: id.clone(),
                write_ability: ability,
            },
            Some(id),
            vec![ability],
            true,
        )
    } else {
        match scan.exact_matches.as_slice() {
            [] => {
                if !scan.unmarked_display_name_ids.is_empty() {
                    return Err(AppError::AdoptionRequired(
                        "workflow identity requires explicit adoption".into(),
                    ));
                }
                (None, ResolutionTarget::Create, None, Vec::new(), false)
            }
            [single] => {
                let list_ability = prove_write(&single.user_abilities, "Workflow")?;
                let detail = fetch_detail(client, &single.id).await?;
                if detail.id != single.id
                    || detail.project != request.project_name
                    || detail.created_by.id != creator_user_id
                {
                    return Err(AppError::ResolutionUnstable(
                        "workflow detail no longer matches the resolved identity".into(),
                    ));
                }
                if classify_marker(
                    detail.meta_config.0.as_deref(),
                    &detail.project,
                    request.project_name,
                    request.slug,
                    &creator_user_id,
                ) != MarkerClassification::Exact
                {
                    return Err(AppError::ResolutionUnstable(
                        "workflow identity changed before projection".into(),
                    ));
                }
                let detail_ability = prove_write(&detail.user_abilities, "Workflow")?;
                let id = detail.id;
                (
                    Some(ExistingEntity {
                        server_id: id.clone(),
                        meta_config: detail.meta_config.0,
                    }),
                    ResolutionTarget::Update {
                        server_id: id.clone(),
                        write_ability: detail_ability,
                    },
                    Some(id),
                    vec![list_ability, detail_ability],
                    false,
                )
            }
            matches => {
                return Err(AppError::Reconciliation(format!(
                    "Workflow: {} matches for (project={:?}, slug={:?}); manual resolution required",
                    matches.len(),
                    request.project_name,
                    request.slug
                )));
            }
        }
    };

    let references = resolve_execution_references(client, request.declaration).await?;

    let mut plan = project_with_workflow_references(
        request.declaration,
        existing_entity.as_ref(),
        Some(&references.map),
    )?;
    bind_creator_marker(&mut plan, &creator_user_id)?;

    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        _slug: request.slug.to_owned(),
        target,
        _scope_scans: scan.evidence,
        _resolved_references: references.map,
        _detail_id: detail_id,
        write_abilities,
        _adoption: adoption,
    };
    let visibility = preflight_visibility(client, request.project_name).await?;
    let prepared = PreparedWrite::workflow(client, visibility, resolution, plan)?;
    dispatch(prepared).await
}

fn bind_creator_marker(plan: &mut WritePlan, creator_user_id: &str) -> Result<(), AppError> {
    let request = match plan {
        WritePlan::Create { request } | WritePlan::Update { request, .. } => request,
    };
    let RequestBody::Json(body) = request else {
        return Err(AppError::Internal(
            "Workflow projection must be JSON".into(),
        ));
    };
    let raw = body
        .get("meta_config")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            AppError::Internal("Workflow projection omitted identity metadata".into())
        })?;
    let mut value: serde_json::Value = crate::strict_json::from_str(raw).map_err(|source| {
        crate::error::ApplicationError::JsonDecoding {
            operation: "workflow identity marker",
            source,
        }
    })?;
    value
        .get_mut(IDENTITY_KEY)
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| AppError::Internal("Workflow projection omitted reserved identity".into()))?
        .insert(
            "creator_user_id".into(),
            serde_json::Value::String(creator_user_id.to_owned()),
        );
    body["meta_config"] =
        serde_json::Value::String(serde_json::to_string(&value).map_err(|source| {
            crate::error::ApplicationError::JsonEncoding {
                operation: "workflow identity marker",
                source,
            }
        })?);
    Ok(())
}

async fn dispatch(prepared: PreparedWrite<'_>) -> Result<ApplyResult, AppError> {
    let action = match prepared.target() {
        ResolutionTarget::Create => ApplyAction::Created,
        ResolutionTarget::Update { .. } => ApplyAction::Updated,
    };
    let response = ApiClient::dispatch_prepared(prepared).await?;
    let response = decode_write_response::<WorkflowWriteResponse>(response)?.ok_or_else(|| {
        AppError::Internal("Workflow modifying request cannot return a conflict signal".into())
    })?;
    ApplyResult::from_server_response(action, response.into_id())
}

/// Re-resolve a Workflow marker after a modifying request and require that it
/// identifies exactly the server route returned by that request (FR-034).
pub async fn verify_identity(
    client: &ApiClient,
    project_name: &str,
    slug: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let membership = preflight_visibility(client, project_name).await?;
    let detail = fetch_detail(client, expected_server_id).await?;
    if detail.id != expected_server_id
        || detail.project != project_name
        || classify_marker(
            detail.meta_config.0.as_deref(),
            project_name,
            project_name,
            slug,
            membership.authenticated_user_id(),
        ) != MarkerClassification::Exact
    {
        return Err(AppError::Reconciliation(
            "Workflow write may have committed but identity verification did not match exactly"
                .into(),
        ));
    }
    Ok(())
}

/// Resolve the single Workflow snapshot selected by `save` using the same
/// exhaustive marker gateway as apply and post-write verification.
pub(crate) async fn resolve_snapshot<T: WorkflowSnapshotContract>(
    client: &ApiClient,
    project_name: &str,
    slug: &str,
    explicit_id: Option<&str>,
) -> Result<T, AppError> {
    let visibility = preflight_visibility(client, project_name).await?;
    let scan = enumerate_all(
        client,
        project_name,
        slug,
        visibility.authenticated_user_id(),
        Some(slug),
    )
    .await?;

    if let Some(explicit_id) = explicit_id {
        if !scan.exact_matches.is_empty() {
            return Err(AppError::Reconciliation(
                "workflow marker conflict requires natural selection".into(),
            ));
        }
        let detail: T = client
            .get(&format!(
                "/v1/workflows/id/{}",
                encode_path_segment(explicit_id)
            ))
            .await?;
        if detail.id() != Some(explicit_id) || detail.project() != Some(project_name) {
            return Err(AppError::ApiIncompatible(
                "workflow detail does not match the explicit selector".into(),
            ));
        }
        if classify_marker(detail.meta_config(), project_name, project_name, "", "")
            != MarkerClassification::Unmarked
        {
            return Err(AppError::WorkflowAlreadyMarked);
        }
        ensure_snapshot_exportable(&detail)?;
        return Ok(detail);
    }

    match scan.exact_matches.as_slice() {
        [single] => {
            ensure_exportable(single)?;
            let detail: T = client
                .get(&format!(
                    "/v1/workflows/id/{}",
                    encode_path_segment(&single.id)
                ))
                .await?;
            if detail.id() != Some(single.id.as_str()) || detail.project() != Some(project_name) {
                return Err(AppError::ApiIncompatible(
                    "workflow detail does not match the natural selector".into(),
                ));
            }
            ensure_snapshot_exportable(&detail)?;
            Ok(detail)
        }
        [] if !scan.unmarked_display_name_ids.is_empty() => Err(AppError::AdoptionRequired(
            "workflow has an unmarked display-name match; use --id".into(),
        )),
        [] => Err(AppError::EntityNotFound),
        _ => Err(AppError::Reconciliation(
            "workflow marker identity is ambiguous".into(),
        )),
    }
}

fn ensure_snapshot_exportable<T: WorkflowSnapshotContract>(detail: &T) -> Result<(), AppError> {
    if detail.mode() == Some("Autonomous") || detail.deprecated() {
        Err(AppError::EntityNotExportable)
    } else {
        Ok(())
    }
}

fn ensure_exportable(workflow: &WorkflowItem) -> Result<(), AppError> {
    if workflow.mode.as_deref() == Some("Autonomous") || workflow.deprecated {
        Err(AppError::EntityNotExportable)
    } else {
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// W-002 execution/reference projection
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct WorkflowReferenceKeys {
    assistants: BTreeSet<(String, String)>,
    skills: BTreeSet<(String, String)>,
    datasources: BTreeSet<(String, String)>,
}

/// Collect and resolve every distinct Workflow server-resource natural key.
/// Workflow-local actor/state IDs are deliberately not collected here.
async fn resolve_execution_references(
    client: &ApiClient,
    decl: &ParsedDeclaration,
) -> Result<ResolvedWorkflowReferences, AppError> {
    let keys = collect_execution_reference_keys(decl)?;
    let mut resolved = WorkflowReferenceMap::default();

    for (project, slug) in keys.assistants {
        let server_id = assistant::resolve_reference(client, &project, &slug).await?;
        resolved.insert_assistant(project, slug, server_id);
    }
    for (project, name) in keys.skills {
        let server_id = skill::resolve_reference(client, &project, &name).await?;
        resolved.insert_skill(project, name, server_id);
    }
    for (project, repo_name) in keys.datasources {
        let server_id = datasource::resolve_reference(client, &project, &repo_name).await?;
        resolved.insert_datasource(project, repo_name, server_id);
    }

    Ok(ResolvedWorkflowReferences { map: resolved })
}

fn collect_execution_reference_keys(
    decl: &ParsedDeclaration,
) -> Result<WorkflowReferenceKeys, AppError> {
    let mut keys = WorkflowReferenceKeys::default();
    let ParsedDeclarationRef::Workflow(workflow) = decl.typed() else {
        #[cfg(test)]
        if let ParsedDeclarationRef::Fixture(crate::parse::EntityKind::Workflow, value) =
            decl.typed()
        {
            return collect_fixture_reference_keys(value);
        }
        return Err(AppError::Schema("workflow declaration is required".into()));
    };
    for actor in &workflow.spec.execution_config.assistants {
        match actor {
            ExecutionConfigAssistantsItem::PersistedWorkflowActor(actor) => {
                keys.assistants.insert((
                    actor.assistant_ref.project.to_string(),
                    actor.assistant_ref.slug.to_string(),
                ));
            }
            ExecutionConfigAssistantsItem::InlineWorkflowActor(actor) => {
                keys.skills.extend(
                    actor.skill_refs.iter().map(|reference| {
                        (reference.project.to_string(), reference.name.to_string())
                    }),
                );
                keys.datasources
                    .extend(actor.datasource_refs.iter().map(|reference| {
                        (
                            reference.project.to_string(),
                            reference.repo_name.to_string(),
                        )
                    }));
            }
        }
    }
    Ok(keys)
}

#[cfg(test)]
fn collect_fixture_reference_keys(
    declaration: &serde_json::Value,
) -> Result<WorkflowReferenceKeys, AppError> {
    let actors = declaration
        .pointer("/spec/execution_config/assistants")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants must be an array".into())
        })?;
    let mut keys = WorkflowReferenceKeys::default();
    for actor in actors {
        let actor = actor.as_object().ok_or_else(|| {
            AppError::Schema("workflow: execution_config.assistants[] must be an object".into())
        })?;
        if let Some(reference) = actor.get("assistantRef") {
            keys.assistants
                .insert(reference_key(reference, "slug", "assistantRef")?);
        } else {
            collect_reference_array(
                actor.get("skillRefs"),
                "name",
                "skillRefs",
                &mut keys.skills,
            )?;
            collect_reference_array(
                actor.get("datasourceRefs"),
                "repo_name",
                "datasourceRefs",
                &mut keys.datasources,
            )?;
        }
    }
    Ok(keys)
}

#[cfg(test)]
fn collect_reference_array(
    value: Option<&serde_json::Value>,
    key_field: &str,
    field: &str,
    output: &mut BTreeSet<(String, String)>,
) -> Result<(), AppError> {
    let values = value.and_then(serde_json::Value::as_array).ok_or_else(|| {
        AppError::Schema(format!("workflow: inline actor {field} must be an array"))
    })?;
    for value in values {
        output.insert(reference_key(value, key_field, field)?);
    }
    Ok(())
}

#[cfg(test)]
fn reference_key(
    value: &serde_json::Value,
    key_field: &str,
    field: &str,
) -> Result<(String, String), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::Schema(format!("workflow: {field} must be an object")))?;
    let project = object
        .get("project")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.project is required")))?;
    let key = object
        .get(key_field)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Schema(format!("workflow: {field}.{key_field} is required")))?;
    Ok((project.to_owned(), key.to_owned()))
}

// ---------------------------------------------------------------------------
// Two-pass enumerate with deduplication
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Explicit adoption: fetch current state for meta_config merge
// ---------------------------------------------------------------------------

async fn fetch_detail(client: &ApiClient, workflow_id: &str) -> Result<WorkflowItem, AppError> {
    let path = format!("/v1/workflows/id/{}", encode_query_value(workflow_id));
    client.get(&path).await
}

// ---------------------------------------------------------------------------
// Identity classification: strict JSON and closed reserved record (ADR-008)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Tests (W-001)
// ---------------------------------------------------------------------------
