/// Skill entity adapter — S-001.
///
/// Exhaustive `(project, name)` resolution via paginated `GET /v1/skills?filters=...`.
/// Zero matches → one POST. A server collision is terminal.
/// One match  → unconditional PUT.
/// Many matches → `AppError::Reconciliation`.
use serde::Deserialize;

use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value, preflight_visibility};

use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, project};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence,
    decode_write_response, prove_write,
};

#[derive(Deserialize)]
struct SkillPage {
    skills: Vec<SkillItem>,
    page: u32,
    #[serde(rename = "perPage")]
    per_page: u32,
    total: u32,
    pages: u32,
}

#[derive(Deserialize, Clone)]
struct SkillItem {
    id: String,
    name: String,
    project: String,
    #[serde(rename = "created_by")]
    created_by: Creator,
    user_abilities: Vec<String>,
}

#[derive(Deserialize, Clone)]
struct Creator {
    id: String,
}

#[derive(Deserialize)]
struct SkillIdResponse {
    id: String,
}

#[derive(Debug)]
struct ScanEvidence {
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

struct Enumeration {
    matches: Vec<SkillItem>,
    evidence: ScanEvidence,
}

/// Concrete exhaustive Skill resolution evidence owned by `PreparedWrite`.
#[derive(Debug)]
pub(super) struct CompletedResolution {
    effective_project: String,
    _name: String,
    target: ResolutionTarget,
    _scan: ScanEvidence,
    write_ability: Option<WriteAbilityEvidence>,
}

impl CompletedResolution {
    pub(super) fn effective_project(&self) -> &str {
        &self.effective_project
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

pub async fn apply(
    client: &ApiClient,
    decl: &ParsedDeclaration,
    project_name: &str,
    skill_name: &str,
) -> Result<ApplyResult, AppError> {
    let initial_visibility = preflight_visibility(client, project_name).await?;
    let creator = initial_visibility.authenticated_user_id().to_owned();
    let enumeration = enumerate(client, project_name, skill_name, &creator).await?;
    match enumeration.matches.as_slice() {
        [] => {
            create_with_reresolution(
                client,
                CreateRequest {
                    declaration: decl,
                    project_name,
                    skill_name,
                    creator_user_id: &creator,
                    initial_scan: enumeration.evidence,
                },
            )
            .await
        }
        [single] => {
            let write_ability = prove_write(&single.user_abilities, "Skill")?;
            let existing = ExistingEntity {
                server_id: single.id.clone(),
                meta_config: None,
            };
            let plan = project(decl, Some(&existing))?;
            let resolution = CompletedResolution {
                effective_project: project_name.to_owned(),
                _name: skill_name.to_owned(),
                target: ResolutionTarget::Update {
                    server_id: single.id.clone(),
                    write_ability,
                },
                _scan: enumeration.evidence,
                write_ability: Some(write_ability),
            };
            let visibility = preflight_visibility(client, project_name).await?;
            let prepared = PreparedWrite::skill(client, visibility, resolution, plan)?;
            dispatch_update(prepared).await
        }
        _ => Err(AppError::Reconciliation(format!(
            "Skill: {} matches for (name={skill_name:?}, project={project_name:?}); \
             manual resolution required",
            enumeration.matches.len()
        ))),
    }
}

/// Resolve a Skill natural reference without creating or updating it
/// (FR-031/DR-003/W-002).
pub async fn resolve_reference(
    client: &ApiClient,
    project_name: &str,
    skill_name: &str,
) -> Result<String, AppError> {
    let visibility = preflight_visibility(client, project_name).await?;
    let enumeration = enumerate(
        client,
        project_name,
        skill_name,
        visibility.authenticated_user_id(),
    )
    .await?;
    match enumeration.matches.as_slice() {
        [single] => Ok(single.id.clone()),
        [] => Err(AppError::MissingReference(
            "referenced Skill is missing on the target server".into(),
        )),
        _ => Err(AppError::Reconciliation(
            "referenced Skill identity is ambiguous on the target server".into(),
        )),
    }
}

/// Post-write exact identity verification for the coordinator (FR-034/R-001).
pub async fn verify_identity(
    client: &ApiClient,
    project_name: &str,
    skill_name: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let actual = resolve_reference(client, project_name, skill_name).await?;
    if actual == expected_server_id {
        Ok(())
    } else {
        Err(AppError::Reconciliation(
            "Skill write may have committed but identity verification did not match".into(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Exhaustive enumerate — all pages, exact client-side filter
// ---------------------------------------------------------------------------

async fn enumerate(
    client: &ApiClient,
    project_name: &str,
    skill_name: &str,
    creator_user_id: &str,
) -> Result<Enumeration, AppError> {
    let filter = serde_json::to_string(&serde_json::json!({
        "project": project_name,
        "scope": "project_with_marketplace",
        "search": skill_name,
    }))
    .map_err(|source| crate::error::ApplicationError::JsonEncoding {
        operation: "skill filter",
        source,
    })?;

    let scan = crate::pagination::scan_pages("skill", |page| {
        let path = format!(
            "/v1/skills?filters={}&page={}&per_page=100",
            encode_query_value(&filter),
            page
        );
        async move {
            let resp: SkillPage = client.get(&path).await?;
            for item in &resp.skills {
                if item.created_by.id.is_empty() {
                    return Err(AppError::ApiIncompatible(
                        "skill creator id is empty".into(),
                    ));
                }
            }
            Ok(crate::pagination::Page {
                items: resp.skills,
                pagination: crate::pagination::PaginationInput {
                    requested_page: page,
                    page: resp.page,
                    per_page: resp.per_page,
                    total: resp.total,
                    pages: resp.pages,
                },
            })
        }
    })
    .await?;

    let matches = scan
        .items
        .into_iter()
        .filter(|item| {
            item.name == skill_name
                && item.project == project_name
                && item.created_by.id == creator_user_id
        })
        .collect();

    Ok(Enumeration {
        matches,
        evidence: ScanEvidence {
            _pages_requested: scan.pages_requested,
            _items_seen: scan.items_seen,
            _advertised_total: scan.advertised_total,
        },
    })
}

impl crate::pagination::PageItem for SkillItem {
    fn page_item_id(&self) -> &str {
        &self.id
    }
}

// ---------------------------------------------------------------------------
// Create
// ---------------------------------------------------------------------------

struct CreateRequest<'a> {
    declaration: &'a ParsedDeclaration,
    project_name: &'a str,
    skill_name: &'a str,
    creator_user_id: &'a str,
    initial_scan: ScanEvidence,
}

async fn create_with_reresolution(
    client: &ApiClient,
    request: CreateRequest<'_>,
) -> Result<ApplyResult, AppError> {
    let plan = project(request.declaration, None)?;
    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        _name: request.skill_name.to_owned(),
        target: ResolutionTarget::Create,
        _scan: request.initial_scan,
        write_ability: None,
    };
    let visibility = preflight_visibility(client, request.project_name).await?;
    let prepared = PreparedWrite::skill(client, visibility, resolution, plan)?;
    match dispatch_create(prepared).await? {
        Some(resp) => ApplyResult::from_server_response(ApplyAction::Created, resp.id),
        None => {
            let collision = enumerate(
                client,
                request.project_name,
                request.skill_name,
                request.creator_user_id,
            )
            .await?;
            match collision.matches.len() {
                1 => Err(AppError::ServerRejected(
                    "Skill create collided with an existing same-creator identity".into(),
                )),
                0 => Err(AppError::ResolutionUnstable(
                    "Skill create conflict could not be resolved by the bounded same-creator scan"
                        .into(),
                )),
                _ => Err(AppError::ResolutionUnstable(
                    "Skill create conflict resolved to multiple same-creator identities".into(),
                )),
            }
        }
    }
}

async fn dispatch_create(prepared: PreparedWrite<'_>) -> Result<Option<SkillIdResponse>, AppError> {
    let response = ApiClient::dispatch_prepared(prepared).await?;
    decode_write_response(response)
}

// ---------------------------------------------------------------------------
// Update dispatch
// ---------------------------------------------------------------------------

async fn dispatch_update(prepared: PreparedWrite<'_>) -> Result<ApplyResult, AppError> {
    if !matches!(prepared.target(), ResolutionTarget::Update { .. }) {
        return Err(AppError::Internal(
            "skill update dispatcher requires update resolution evidence".into(),
        ));
    }
    let response = ApiClient::dispatch_prepared(prepared).await?;
    let response: SkillIdResponse = decode_write_response(response)?
        .ok_or_else(|| AppError::Internal("Skill update cannot return a conflict signal".into()))?;
    ApplyResult::from_server_response(ApplyAction::Updated, response.id)
}

// ---------------------------------------------------------------------------
// Tests (S-001)
// ---------------------------------------------------------------------------
