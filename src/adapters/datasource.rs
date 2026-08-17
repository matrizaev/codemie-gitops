/// Datasource entity adapter — D-001.
///
/// Visibility preflight (GET /v1/user) required before any write.
/// Exhaustive (project, repo_name, index_type) identity resolution via
/// paginated GET /v1/index?full_response=true&page={page}&per_page=100&filters={json}.
/// The persisted kind participates in identity matching: a visible row selects
/// update only when its kind equals the declaration's kind; a miss (including
/// a same-name row of a different kind) permits one create. The kind is read
/// from the server's index_type/vcs_type fields (see server_kind_matches).
/// Pagination is zero-indexed. Cap: 1,000 pages / 100,000 items → E_API_INCOMPATIBLE.
/// File Datasource: multipart transport with parts cap (10) and basename safety.
use serde::Deserialize;

use crate::error::AppError;

/// Extra re-polls after a create response before declaring the write uncertain.
///
/// The server acknowledges datasource creates before the index row is
/// committed by its background indexing task; a single immediate re-read can
/// still observe an empty page. Bounded, short re-polls make the create path
/// deterministic without unbounded waiting.
const POST_WRITE_REPOLL_ATTEMPTS: u32 = 4;

/// Delay between post-write re-polls in milliseconds.
const POST_WRITE_REPOLL_INTERVAL_MS: u64 = 400;
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
    #[serde(rename = "vcs_type", default)]
    vcs_type: Option<String>,
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
        // The create response is acknowledged before the index row is
        // committed (background indexing task); re-poll briefly instead of
        // declaring the write uncertain on the first, still-empty page.
        let mut post_write = None;
        for attempt in 0..=POST_WRITE_REPOLL_ATTEMPTS {
            let current = enumerate(
                client,
                request.project_name,
                request.repo_name,
                request.index_type,
            )
            .await?;
            match current.matches.as_slice() {
                [single] => {
                    post_write = Some(single.id.clone());
                    break;
                }
                [] if attempt < POST_WRITE_REPOLL_ATTEMPTS => {
                    tokio::time::sleep(std::time::Duration::from_millis(
                        POST_WRITE_REPOLL_INTERVAL_MS,
                    ))
                    .await;
                }
                _ => break,
            }
        }
        match post_write {
            Some(id) => id,
            None => {
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
                    && server_kind_matches(item, index_type)
            })
            .collect(),
        evidence: project.evidence,
    })
}

/// Whether a server index row carries the declaration's Datasource kind.
///
/// Servers historically reported the kind directly in the index_type field;
/// current servers store the index granularity (code/summary/chunk-summary)
/// there for VCS kinds and move the kind to vcs_type, while knowledge-base
/// kinds remain prefixed in index_type (knowledge_base_*,
/// llm_routing_google). This mirrors the server-to-declaration mapping in
/// src/save/reverse.rs and keeps identity resolution exact across both
/// shapes without ever treating a different kind as a match.
fn server_kind_matches(item: &DatasourceItem, kind: &str) -> bool {
    match item.index_type.as_str() {
        "code" | "summary" | "chunk-summary" => {
            item.vcs_type.as_deref() == Some(kind) && matches!(kind, "git" | "svn")
        }
        "knowledge_base_confluence" => kind == "confluence",
        "knowledge_base_jira" => kind == "jira",
        "knowledge_base_xray" => kind == "xray",
        "knowledge_base_azure_devops_wiki" => kind == "azure_devops_wiki",
        "knowledge_base_azure_devops_work_item" => kind == "azure_devops_work_item",
        "knowledge_base_sharepoint" => kind == "sharepoint",
        "llm_routing_google" => kind == "google",
        "knowledge_base_file" => kind == "file",
        other => other == kind,
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn item(index_type: &str, vcs_type: Option<&str>) -> DatasourceItem {
        DatasourceItem {
            id: "id".to_owned(),
            repo_name: "repo".to_owned(),
            project_name: "project".to_owned(),
            index_type: index_type.to_owned(),
            vcs_type: vcs_type.map(str::to_owned),
            user_abilities: vec![],
        }
    }

    #[test]
    fn legacy_server_reports_kind_directly_in_index_type() {
        // Historical shape: the declaration kind was the persisted value.
        assert!(server_kind_matches(&item("git", None), "git"));
        assert!(server_kind_matches(&item("svn", None), "svn"));
        assert!(server_kind_matches(&item("google", None), "google"));
        assert!(!server_kind_matches(&item("git", None), "svn"));
    }

    #[test]
    fn vcs_kinds_read_kind_from_vcs_type_when_index_type_is_granularity() {
        // Current shape: VCS rows carry the granularity in index_type and the
        // kind in vcs_type.
        assert!(server_kind_matches(&item("code", Some("git")), "git"));
        assert!(server_kind_matches(&item("summary", Some("svn")), "svn"));
        assert!(!server_kind_matches(&item("code", Some("git")), "svn"));
        // A granularity without a matching VCS kind is never a match.
        assert!(!server_kind_matches(
            &item("code", Some("git")),
            "confluence"
        ));
        assert!(!server_kind_matches(&item("code", None), "git"));
    }

    #[test]
    fn knowledge_base_kinds_keep_prefixed_index_type() {
        assert!(server_kind_matches(
            &item("knowledge_base_confluence", None),
            "confluence"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_jira", None),
            "jira"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_xray", None),
            "xray"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_azure_devops_wiki", None),
            "azure_devops_wiki"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_azure_devops_work_item", None),
            "azure_devops_work_item"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_sharepoint", None),
            "sharepoint"
        ));
        assert!(server_kind_matches(
            &item("llm_routing_google", None),
            "google"
        ));
        assert!(server_kind_matches(
            &item("knowledge_base_file", None),
            "file"
        ));
        // A different prefixed kind never matches.
        assert!(!server_kind_matches(
            &item("knowledge_base_jira", None),
            "confluence"
        ));
    }
}
