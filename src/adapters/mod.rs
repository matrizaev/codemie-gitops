/// Entity adapter module: one adapter per entity kind.
///
/// All adapters share the same write policy (ADR-002):
/// - Absent identity → POST/`created`.
/// - Present identity → unconditional PUT/`updated`.
/// - No field-state-dependent write suppression.
/// - Server IDs are invocation-local only and never output.
pub mod assistant;
pub mod datasource;
pub mod skill;
pub mod workflow;

use std::num::NonZeroUsize;

use crate::domain::ServerId;
use crate::error::AppError;
use crate::http::{ExactProjectVisibility, encode_query_value};
use crate::pagination::PaginationError;
use crate::parse::EntityKind;
use crate::projection::{RequestBody, WritePlan};

fn map_pagination_error(entity: &str, error: PaginationError) -> AppError {
    if error.is_drift() {
        AppError::Reconciliation(format!("{entity} {error}"))
    } else {
        AppError::ApiIncompatible(format!("{entity} {error}"))
    }
}

/// The identity-resolution outcome to which a projected request must be linked.
///
/// This value is never constructed by the generic seal. Each adapter creates it
/// only from a successful strict read: an absent direct lookup/full scan or one
/// exact, writable row.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolutionTarget {
    Create,
    Update { server_id: String },
}

impl ResolutionTarget {
    fn matches_plan(&self, plan: &WritePlan) -> bool {
        match (self, plan) {
            (Self::Create, WritePlan::Create { .. }) => true,
            (
                Self::Update {
                    server_id: evidence_id,
                },
                WritePlan::Update { server_id, .. },
            ) => evidence_id == server_id,
            _ => false,
        }
    }
}

/// Concrete proof returned by strict row-ability validation.
///
/// The non-zero count makes this actual consumed response evidence instead of a
/// fabricated completion marker. Construction remains private to `prove_write`.
#[derive(Debug, Clone)]
struct WriteAbilityEvidence {
    _decoded_ability_count: NonZeroUsize,
}

/// Operation-specific capability evidence established before identity reads.
#[derive(Debug)]
enum OperationPreflight {
    ExactProjectVisibility(ExactProjectVisibility),
}

/// Completed kind-specific read evidence. Every variant owns values returned by
/// the corresponding adapter's successful strict reads.
#[derive(Debug)]
enum CompletedResolution {
    Assistant(assistant::CompletedResolution),
    Workflow(workflow::CompletedResolution),
    Skill(skill::CompletedResolution),
    Datasource(datasource::CompletedResolution),
}

impl CompletedResolution {
    fn kind(&self) -> EntityKind {
        match self {
            Self::Assistant(_) => EntityKind::Assistant,
            Self::Workflow(_) => EntityKind::Workflow,
            Self::Skill(_) => EntityKind::Skill,
            Self::Datasource(_) => EntityKind::Datasource,
        }
    }

    fn effective_project(&self) -> &str {
        match self {
            Self::Assistant(evidence) => evidence.effective_project(),
            Self::Workflow(evidence) => evidence.effective_project(),
            Self::Skill(evidence) => evidence.effective_project(),
            Self::Datasource(evidence) => evidence.effective_project(),
        }
    }

    fn target(&self) -> &ResolutionTarget {
        match self {
            Self::Assistant(evidence) => evidence.target(),
            Self::Workflow(evidence) => evidence.target(),
            Self::Skill(evidence) => evidence.target(),
            Self::Datasource(evidence) => evidence.target(),
        }
    }
}

/// The only value accepted by the production modifying transport boundary.
///
/// Fields and constructors are private to the adapter tree. The HTTP transport
/// can consume this value, but no caller can manufacture it from a raw path and
/// body. It owns capability, completed reads, projected request, and (for File
/// Datasource) the successfully read multipart bytes.
pub(crate) struct PreparedWrite<'a> {
    client: &'a crate::http::ApiClient,
    plan: WritePlan,
    evidence: PrewriteEvidence,
    file_parts: Option<Vec<(String, Vec<u8>)>>,
}

#[derive(Debug)]
struct PrewriteEvidence {
    operation_preflight: OperationPreflight,
    resolution: CompletedResolution,
}

/// Closed modifying methods understood by the internal transport dispatcher.
pub(crate) enum ModificationMethod {
    Post,
    Put,
}

/// Fully projected request released only when `PreparedWrite` is consumed.
pub(crate) enum PreparedRequest {
    Json {
        method: ModificationMethod,
        path: String,
        body: serde_json::Value,
        conflict_is_resolution_signal: bool,
    },
    Multipart {
        method: ModificationMethod,
        path: String,
        query_params: Vec<(String, String)>,
        file_parts: Vec<(String, Vec<u8>)>,
    },
}

/// Result of the single evidence-bearing modifying boundary.
pub(crate) enum PreparedWriteResponse {
    Success(OpenWriteResponse),
    Conflict,
}

/// Opaque modifying-response envelope owned by the transport boundary.
///
/// Entity adapters can only consume it through typed deserialization; the
/// open JSON representation cannot leak into application orchestration.
pub(crate) struct OpenWriteResponse(serde_json::Value);

impl From<serde_json::Value> for OpenWriteResponse {
    fn from(value: serde_json::Value) -> Self {
        Self(value)
    }
}

impl<'a> PreparedWrite<'a> {
    fn target(&self) -> &ResolutionTarget {
        self.evidence.resolution.target()
    }

    fn assistant(
        client: &'a crate::http::ApiClient,
        visibility: ExactProjectVisibility,
        resolution: assistant::CompletedResolution,
        plan: WritePlan,
    ) -> Result<Self, AppError> {
        Self::visible(
            client,
            visibility,
            CompletedResolution::Assistant(resolution),
            plan,
            None,
        )
    }

    fn workflow(
        client: &'a crate::http::ApiClient,
        visibility: ExactProjectVisibility,
        resolution: workflow::CompletedResolution,
        plan: WritePlan,
    ) -> Result<Self, AppError> {
        Self::visible(
            client,
            visibility,
            CompletedResolution::Workflow(resolution),
            plan,
            None,
        )
    }

    fn skill(
        client: &'a crate::http::ApiClient,
        visibility: ExactProjectVisibility,
        resolution: skill::CompletedResolution,
        plan: WritePlan,
    ) -> Result<Self, AppError> {
        Self::visible(
            client,
            visibility,
            CompletedResolution::Skill(resolution),
            plan,
            None,
        )
    }

    fn datasource(
        client: &'a crate::http::ApiClient,
        visibility: ExactProjectVisibility,
        resolution: datasource::CompletedResolution,
        plan: WritePlan,
        file_parts: Option<Vec<(String, Vec<u8>)>>,
    ) -> Result<Self, AppError> {
        Self::visible(
            client,
            visibility,
            CompletedResolution::Datasource(resolution),
            plan,
            file_parts,
        )
    }

    fn visible(
        client: &'a crate::http::ApiClient,
        visibility: ExactProjectVisibility,
        resolution: CompletedResolution,
        plan: WritePlan,
        file_parts: Option<Vec<(String, Vec<u8>)>>,
    ) -> Result<Self, AppError> {
        let effective_project = resolution.effective_project();
        if !visibility.matches(effective_project) {
            return Err(AppError::Internal(
                "visibility proof project does not match prepared write".into(),
            ));
        }
        Self::seal(
            client,
            OperationPreflight::ExactProjectVisibility(visibility),
            resolution,
            plan,
            file_parts,
        )
    }

    fn seal(
        client: &'a crate::http::ApiClient,
        operation_preflight: OperationPreflight,
        resolution: CompletedResolution,
        plan: WritePlan,
        file_parts: Option<Vec<(String, Vec<u8>)>>,
    ) -> Result<Self, AppError> {
        if resolution.effective_project().is_empty() {
            return Err(AppError::Internal(
                "prepared write requires a non-empty effective project".into(),
            ));
        }
        if !resolution.target().matches_plan(&plan) {
            return Err(AppError::Internal(
                "projected request is not linked to completed resolution evidence".into(),
            ));
        }
        Ok(Self {
            client,
            plan,
            evidence: PrewriteEvidence {
                operation_preflight,
                resolution,
            },
            file_parts,
        })
    }

    /// Validate and consume the seal, yielding the sole request shape accepted
    /// by the raw HTTP send primitives.
    pub(crate) fn into_request(
        self,
    ) -> Result<(&'a crate::http::ApiClient, PreparedRequest), AppError> {
        let client = self.client;
        let PrewriteEvidence {
            operation_preflight,
            resolution,
        } = self.evidence;
        let kind = resolution.kind();
        let project = resolution.effective_project().to_owned();
        match operation_preflight {
            OperationPreflight::ExactProjectVisibility(visibility)
                if visibility.matches(&project) => {}
            _ => {
                return Err(AppError::Internal(
                    "prepared write evidence does not match its entity kind or project".into(),
                ));
            }
        }
        if !resolution.target().matches_plan(&self.plan) {
            return Err(AppError::Internal(
                "prepared write plan no longer matches resolution evidence".into(),
            ));
        }

        let (method, server_id, request) = match self.plan {
            WritePlan::Create { request } => (ModificationMethod::Post, None, request),
            WritePlan::Update { server_id, request } => {
                (ModificationMethod::Put, Some(server_id), request)
            }
        };

        let path = match (&resolution, &server_id) {
            (CompletedResolution::Assistant(_), None) => "/v1/assistants".to_owned(),
            (CompletedResolution::Assistant(_), Some(id)) => {
                format!("/v1/assistants/{}", encode_query_value(id))
            }
            (CompletedResolution::Workflow(_), None) => "/v1/workflows".to_owned(),
            (CompletedResolution::Workflow(_), Some(id)) => {
                format!("/v1/workflows/{}", encode_query_value(id))
            }
            (CompletedResolution::Skill(_), None) => "/v1/skills".to_owned(),
            (CompletedResolution::Skill(_), Some(id)) => {
                format!("/v1/skills/{}", encode_query_value(id))
            }
            (CompletedResolution::Datasource(evidence), None) => {
                datasource_create_route(evidence.index_type(), &project)
            }
            (CompletedResolution::Datasource(evidence), Some(_)) => {
                datasource_update_route(evidence.index_type(), &project, evidence.repo_name())
            }
        };

        let request = match request {
            RequestBody::Json(body) => {
                if self.file_parts.is_some() {
                    return Err(AppError::Internal(
                        "JSON prepared write unexpectedly owns multipart bytes".into(),
                    ));
                }
                Ok::<PreparedRequest, AppError>(PreparedRequest::Json {
                    conflict_is_resolution_signal: matches!(
                        kind,
                        EntityKind::Skill | EntityKind::Datasource
                    ) && matches!(method, ModificationMethod::Post),
                    method,
                    path,
                    body,
                })
            }
            RequestBody::FileMultipart {
                mut query_params, ..
            } => {
                let CompletedResolution::Datasource(evidence) = &resolution else {
                    return Err(AppError::Internal(
                        "multipart projection is valid only for Datasource".into(),
                    ));
                };
                if matches!(method, ModificationMethod::Put)
                    && !query_params.iter().any(|(key, _)| key == "name")
                {
                    query_params.push(("name".to_owned(), evidence.repo_name().to_owned()));
                }
                let file_parts = self.file_parts.ok_or_else(|| {
                    AppError::Internal(
                        "multipart prepared write requires successfully read file bytes".into(),
                    )
                })?;
                Ok(PreparedRequest::Multipart {
                    method,
                    path,
                    query_params,
                    file_parts,
                })
            }
        }?;
        Ok((client, request))
    }
}

fn datasource_create_route(kind: &str, project: &str) -> String {
    match kind {
        "git" => format!("/v1/application/{}/index", encode_query_value(project)),
        "svn" => format!("/v1/application/{}/index/svn", encode_query_value(project)),
        _ => format!("/v1/index/knowledge_base/{}", encode_query_value(kind)),
    }
}

fn datasource_update_route(kind: &str, project: &str, repo_name: &str) -> String {
    match kind {
        "git" => format!(
            "/v1/application/{}/index/{}",
            encode_query_value(project),
            encode_query_value(repo_name)
        ),
        "svn" => format!(
            "/v1/application/{}/index/svn/{}",
            encode_query_value(project),
            encode_query_value(repo_name)
        ),
        _ => format!("/v1/index/knowledge_base/{}", encode_query_value(kind)),
    }
}

/// Require and retain source-pinned row-level write ability evidence.
fn prove_write(abilities: &[String], entity: &str) -> Result<WriteAbilityEvidence, AppError> {
    if abilities.iter().any(|ability| ability == "write") {
        let decoded_ability_count = NonZeroUsize::new(abilities.len()).ok_or_else(|| {
            AppError::Internal("write ability evidence cannot have an empty vector".into())
        })?;
        Ok(WriteAbilityEvidence {
            _decoded_ability_count: decoded_ability_count,
        })
    } else {
        Err(AppError::Authorization(format!(
            "{entity} target does not expose write capability"
        )))
    }
}

fn decode_write_response<T: serde::de::DeserializeOwned>(
    response: PreparedWriteResponse,
) -> Result<Option<T>, AppError> {
    match response {
        PreparedWriteResponse::Success(value) => serde_json::from_value(value.0)
            .map(Some)
            .map_err(crate::http::TransportError::ResponseShape)
            .map_err(AppError::from),
        PreparedWriteResponse::Conflict => Ok(None),
    }
}

/// What a successful apply operation did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyAction {
    Created,
    Updated,
}

/// The result of a successful single-entity apply.
#[derive(Debug)]
pub struct ApplyResult {
    action: ApplyAction,
    /// Server UUID (never forwarded to logs or user-visible output, SEC-005).
    server_id: ServerId,
}

impl ApplyResult {
    pub(crate) fn from_server_response(
        action: ApplyAction,
        server_id: String,
    ) -> Result<Self, AppError> {
        Ok(Self {
            action,
            server_id: ServerId::try_from(server_id).map_err(|_| {
                AppError::ApiIncompatible("successful response contains an empty server ID".into())
            })?,
        })
    }

    pub(crate) fn action(&self) -> ApplyAction {
        self.action
    }

    pub(crate) fn server_id(&self) -> &str {
        self.server_id.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_resolution_evidence_is_kind_specific_and_not_zero_sized() {
        assert!(std::mem::size_of::<assistant::CompletedResolution>() > 0);
        assert!(std::mem::size_of::<workflow::CompletedResolution>() > 0);
        assert!(std::mem::size_of::<skill::CompletedResolution>() > 0);
        assert!(std::mem::size_of::<datasource::CompletedResolution>() > 0);
        assert!(std::mem::size_of::<PreparedWrite>() > 0);
    }

    #[test]
    fn production_http_api_exposes_only_the_prepared_modifying_boundary() {
        let http_source = include_str!("../http/mod.rs");
        for raw_entry in [
            "pub async fn post<",
            "pub async fn put<",
            "pub async fn post_or_conflict<",
            "pub async fn post_multipart(",
            "pub async fn put_multipart(",
        ] {
            assert!(
                !http_source.contains(raw_entry),
                "raw modifying API escaped its private transport boundary: {raw_entry}"
            );
        }
        assert!(http_source.contains("pub(crate) async fn dispatch_prepared("));

        for adapter_source in [
            include_str!("assistant.rs"),
            include_str!("workflow.rs"),
            include_str!("skill.rs"),
            include_str!("datasource.rs"),
        ] {
            for bypass in [
                ".post(",
                ".put(",
                ".post_or_conflict(",
                ".post_multipart(",
                ".put_multipart(",
            ] {
                assert!(
                    !adapter_source.contains(bypass),
                    "adapter bypasses evidence-bearing dispatcher via {bypass}"
                );
            }
        }
    }

    #[test]
    fn manifest_pins_additive_policy_capability_scope_and_page_bases() {
        let manifest: serde_json::Value = serde_json::from_str(include_str!(
            "../../specs/codemie-cicd-tool/contracts/adapter-manifest-v2.42.0.json"
        ))
        .expect("checked-in adapter manifest must be valid JSON");

        assert_eq!(
            manifest
                .pointer("/compatibilityPolicy/unknownConsumedResponseField")
                .and_then(|v| v.as_str()),
            Some("ignore-only-when-additive-and-unconsumed")
        );
        assert_eq!(
            manifest
                .pointer("/capabilityPreflight/appliesToEntityKinds")
                .and_then(serde_json::Value::as_array)
                .expect("capability applicability must be an array"),
            serde_json::json!(["Assistant", "Workflow", "Datasource", "Skill"])
                .as_array()
                .expect("expected fixture must be an array")
        );
        for pointer in [
            "/entities/Workflow/pagination/pageBase",
            "/entities/Skill/pagination/pageBase",
        ] {
            assert_eq!(
                manifest
                    .pointer(pointer)
                    .and_then(serde_json::Value::as_u64),
                Some(0),
                "{pointer} must remain zero-indexed"
            );
        }
    }
}
