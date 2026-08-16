/// Datasource entity adapter — D-001.
///
/// Visibility preflight (`GET /v1/user`) required before any write.
/// Exhaustive `(project, repo_name, index_type)` identity resolution via
/// paginated `GET /v1/index?full_response=true&page={page}&per_page=100&filters={json}`.
/// The persisted `index_type` participates in identity matching: a visible row
/// selects update only when its kind equals the declaration's kind; a miss
/// (including a same-name row of a different kind) permits one create.
/// Pagination is zero-indexed. Cap: 1,000 pages / 100,000 items → `E_API_INCOMPATIBLE`.
/// File Datasource: multipart transport with parts cap (10) and basename safety.
use serde::Deserialize;

use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value, preflight_visibility};

use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, RequestBody, project};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, PreparedWriteResponse, ResolutionTarget,
    WriteAbilityEvidence, prove_write,
};

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct DatasourcePage {
    data: Vec<DatasourceItem>,
    pagination: DatasourcePagination,
}

#[derive(Deserialize)]
struct DatasourcePagination {
    page: u32,
    per_page: u32,
    total: u32,
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct DatasourceItem {
    id: String,
    repo_name: String,
    project_name: String,
    #[serde(rename = "index_type")]
    index_type: String,
    user_abilities: Vec<String>,
}

#[derive(Deserialize)]
struct DatasourceWriteResponse {
    id: Option<String>,
}

#[derive(Debug)]
struct ScanEvidence {
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

struct ProjectEnumeration {
    items: Vec<DatasourceItem>,
    evidence: ScanEvidence,
}

struct Enumeration {
    matches: Vec<DatasourceItem>,
    evidence: ScanEvidence,
}

/// Concrete exhaustive Datasource resolution evidence owned by `PreparedWrite`.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    repo_name: String,
    index_type: String,
    target: ResolutionTarget,
    _scan: ScanEvidence,
    write_ability: Option<WriteAbilityEvidence>,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
    }

    pub(super) fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub(super) fn index_type(&self) -> &str {
        &self.index_type
    }

    pub(super) fn target(&self) -> &ResolutionTarget {
        &self.target
    }

    pub(super) fn write_ability_is_proven(&self) -> bool {
        self.write_ability.is_some()
    }
}

// ---------------------------------------------------------------------------
// Public adapter entry point
// ---------------------------------------------------------------------------

pub struct ApplyRequest<'a> {
    pub declaration: &'a ParsedDeclaration,
    pub project_name: &'a str,
    pub repo_name: &'a str,
    pub index_type: &'a str,
    pub file_parts: Vec<crate::input::FilePart>,
}

/// Apply one Datasource using bytes validated at the selected-input boundary.
pub async fn apply(client: &ApiClient, request: ApplyRequest<'_>) -> Result<ApplyResult, AppError> {
    // ADR-012 Option A: exact-effective-project preflight before any write.
    let _initial_visibility = preflight_visibility(client, request.project_name).await?;

    let enumeration = enumerate(
        client,
        request.project_name,
        request.repo_name,
        request.index_type,
    )
    .await?;

    let (existing, target, write_ability) = match enumeration.matches.as_slice() {
        [] => (None, ResolutionTarget::Create, None),
        [single] => {
            let write_ability = prove_write(&single.user_abilities, "Datasource")?;
            (
                Some(ExistingEntity {
                    server_id: single.id.clone(),
                    meta_config: None,
                }),
                ResolutionTarget::Update {
                    server_id: single.id.clone(),
                    write_ability,
                },
                Some(write_ability),
            )
        }
        _ => Err(AppError::Reconciliation(format!(
            "Datasource: {} matches for (repo_name={:?}, project={:?}, type={:?}); manual resolution required",
            enumeration.matches.len(),
            request.repo_name,
            request.project_name,
            request.index_type
        )))?,
    };

    let plan = project(request.declaration, existing.as_ref())?;
    let file_parts = if matches!(
        &plan,
        crate::projection::WritePlan::Create {
            request: RequestBody::FileMultipart { .. }
        } | crate::projection::WritePlan::Update {
            request: RequestBody::FileMultipart { .. },
            ..
        }
    ) {
        Some(
            request
                .file_parts
                .into_iter()
                .map(|part| (part.filename, part.bytes))
                .collect(),
        )
    } else {
        None
    };
    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        repo_name: request.repo_name.to_owned(),
        index_type: request.index_type.to_owned(),
        target,
        _scan: enumeration.evidence,
        write_ability,
    };
    let visibility = preflight_visibility(client, request.project_name).await?;
    let prepared = PreparedWrite::datasource(client, visibility, resolution, plan, file_parts)?;
    let dispatched = dispatch(prepared).await?;
    let server_id = if let Some(server_id) = dispatched.server_id {
        server_id
    } else {
        let post_write = enumerate(
            client,
            request.project_name,
            request.repo_name,
            request.index_type,
        )
        .await?;
        match post_write.matches.as_slice() {
            [single] => single.id.clone(),
            _ => {
                return Err(AppError::WriteUncertain(
                    "Datasource write succeeded but its exact server identity could not be re-resolved"
                        .into(),
                ));
            }
        }
    };
    ApplyResult::from_server_response(dispatched.action, server_id)
}

/// Resolve a Datasource natural reference without requiring the target
/// declaration's `index_type` (DR-003/W-002).
///
/// Workflow references identify Datasources by exact `(project, repo_name)`.
/// If more than one visible server row has that identity, including rows of
/// different persisted kinds, resolution is ambiguous and no ID is selected.
pub async fn resolve_reference(
    client: &ApiClient,
    project_name: &str,
    repo_name: &str,
) -> Result<String, AppError> {
    preflight_visibility(client, project_name).await?;
    let matches: Vec<String> = enumerate_project(client, project_name)
        .await?
        .items
        .into_iter()
        .filter(|item| item.repo_name == repo_name && item.project_name == project_name)
        .map(|item| item.id)
        .collect();

    match matches.as_slice() {
        [single] => Ok(single.clone()),
        [] => Err(AppError::MissingReference(
            "referenced Datasource is missing on the target server".into(),
        )),
        _ => Err(AppError::Reconciliation(
            "referenced Datasource identity is ambiguous on the target server".into(),
        )),
    }
}

/// Post-write exact identity verification for the coordinator (R-001).
pub async fn verify_identity(
    client: &ApiClient,
    project_name: &str,
    repo_name: &str,
    index_type: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let enumeration = enumerate(client, project_name, repo_name, index_type).await?;
    match enumeration.matches.as_slice() {
        [single] if single.id == expected_server_id => Ok(()),
        _ => Err(AppError::Reconciliation(
            "Datasource write may have committed but identity verification did not match exactly once"
                .into(),
        )),
    }
}

// ---------------------------------------------------------------------------
// Exhaustive enumerate — zero-indexed pages, exact client-side filter
// ---------------------------------------------------------------------------

async fn enumerate(
    client: &ApiClient,
    project_name: &str,
    repo_name: &str,
    index_type: &str,
) -> Result<Enumeration, AppError> {
    let project = enumerate_project(client, project_name).await?;
    Ok(Enumeration {
        matches: project
            .items
            .into_iter()
            .filter(|item| {
                item.repo_name == repo_name
                    && item.project_name == project_name
                    && item.index_type == index_type
            })
            .collect(),
        evidence: project.evidence,
    })
}

async fn enumerate_project(
    client: &ApiClient,
    project_name: &str,
) -> Result<ProjectEnumeration, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({ "project": project_name })).map_err(
        |source| crate::error::ApplicationError::JsonEncoding {
            operation: "datasource filter",
            source,
        },
    )?;

    let scan = crate::pagination::scan_pages("datasource", |page| {
        let path = format!(
            "/v1/index?full_response=true&page={}&per_page=100&filters={}",
            page,
            encode_query_value(&filter)
        );
        async move {
            let resp: DatasourcePage = client.get(&path).await?;
            Ok(crate::pagination::Page {
                items: resp.data,
                pagination: crate::pagination::PaginationInput {
                    requested_page: page,
                    page: resp.pagination.page,
                    per_page: resp.pagination.per_page,
                    total: resp.pagination.total,
                    pages: resp.pagination.pages,
                },
            })
        }
    })
    .await?;
    Ok(ProjectEnumeration {
        items: scan.items,
        evidence: ScanEvidence {
            _pages_requested: scan.pages_requested,
            _items_seen: scan.items_seen,
            _advertised_total: scan.advertised_total,
        },
    })
}

impl crate::pagination::PageItem for DatasourceItem {
    fn page_item_id(&self) -> &str {
        &self.id
    }
}

// ---------------------------------------------------------------------------
// Dispatch: the HTTP client accepts only the evidence-bearing aggregate
// ---------------------------------------------------------------------------

struct DispatchResult {
    action: ApplyAction,
    server_id: Option<String>,
}

async fn dispatch(prepared: PreparedWrite<'_>) -> Result<DispatchResult, AppError> {
    let action = match prepared.target() {
        ResolutionTarget::Create => ApplyAction::Created,
        ResolutionTarget::Update { .. } => ApplyAction::Updated,
    };
    let response = match ApiClient::dispatch_prepared(prepared).await? {
        PreparedWriteResponse::Conflict => {
            return Err(AppError::ServerRejected(
                "Datasource create collided with an existing server identity".into(),
            ));
        }
        success => success,
    };
    let response: DatasourceWriteResponse = super::decode_write_response(response)?
        .ok_or_else(|| AppError::Internal("Datasource write response was a conflict".into()))?;
    Ok(DispatchResult {
        action,
        server_id: response.id,
    })
}

// ---------------------------------------------------------------------------
// File Datasource basename safety is enforced by the explicit-input loader.
// ---------------------------------------------------------------------------
