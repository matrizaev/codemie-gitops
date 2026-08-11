/// Workflow entity adapter — W-001.
///
/// Identity resolved from `meta_config["codemie.epam.com/gitops/workflow-identity"]`
/// = `{version:1, project, slug}`. Two-pass enumeration (pass 1: project-visible,
/// pass 2: `scope=marketplace`). Deduplicates across passes by server ID.
/// Optional `adopt_workflow_id` is considered only after both scans prove zero
/// exact markers, then validates one explicit by-ID adoption candidate.
use std::collections::BTreeSet;
use std::path::Path;

use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

use crate::config::ValidatedUrl;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value, preflight_visibility};
use crate::parse::ParsedDeclaration;
use crate::projection::{ExistingEntity, WorkflowReferenceMap, project_with_workflow_references};

use super::{
    ApplyAction, ApplyResult, PreparedWrite, ResolutionTarget, WriteAbilityEvidence, assistant,
    datasource, decode_write_response, prove_write, skill,
};

const MAX_PAGES: u32 = 1_000;
const MAX_ITEMS: u32 = 100_000;

const IDENTITY_KEY: &str = "codemie.epam.com/gitops/workflow-identity";

// ---------------------------------------------------------------------------
// Server response shapes
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct WorkflowPage {
    data: Vec<WorkflowItem>,
    pagination: WorkflowPagination,
}

#[derive(Deserialize)]
struct WorkflowPagination {
    page: u32,
    pages: u32,
    total: u32,
    per_page: u32,
}

#[derive(Deserialize, Clone)]
struct WorkflowItem {
    id: String,
    project: String,
    #[serde(rename = "name")]
    name: String,
    meta_config: RequiredNullableString,
    user_abilities: Vec<String>,
}

/// `meta_config` is nullable in the pinned API but its response member is
/// required. A newtype preserves the difference between explicit `null` and
/// an omitted field during Serde struct decoding.
#[derive(Clone)]
struct RequiredNullableString(Option<String>);

impl<'de> Deserialize<'de> for RequiredNullableString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::Null => Ok(Self(None)),
            serde_json::Value::String(value) => Ok(Self(Some(value))),
            _ => Err(serde::de::Error::custom(
                "meta_config must be a string or null",
            )),
        }
    }
}

#[derive(Deserialize)]
struct WorkflowIdResponse {
    id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MarkerClassification {
    Unmarked,
    Exact,
    OtherValid,
    Invalid,
}

#[derive(Debug)]
struct PassEvidence {
    _scope: Option<&'static str>,
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

struct PassResult {
    exact_matches: Vec<WorkflowItem>,
    unmarked_display_name_ids: Vec<String>,
    evidence: PassEvidence,
}

struct WorkflowScan {
    exact_matches: Vec<WorkflowItem>,
    unmarked_display_name_ids: Vec<String>,
    evidence: Vec<PassEvidence>,
}

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
    _write_abilities: Vec<WriteAbilityEvidence>,
    _adoption: bool,
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
// Public adapter entry point
// ---------------------------------------------------------------------------

pub struct ApplyRequest<'a> {
    pub declaration: &'a ParsedDeclaration,
    pub project_name: &'a str,
    pub slug: &'a str,
    pub adopt_workflow_id: Option<&'a str>,
    pub repo_root: &'a Path,
    pub follow_symlinks: bool,
}

pub async fn apply(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    request: ApplyRequest<'_>,
) -> Result<ApplyResult, AppError> {
    let visibility = preflight_visibility(client, base_url, request.project_name).await?;
    let authored_display_name = request
        .declaration
        .value
        .pointer("/spec/name")
        .and_then(serde_json::Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| AppError::Schema("workflow: spec.name is required".into()))?;
    // Both scopes are always exhausted before either ordinary selection or
    // explicit adoption. This proves the zero/exact/ambiguous marker state and
    // fails closed on invalid target-project metadata.
    let scan = enumerate_all(
        client,
        base_url,
        request.project_name,
        request.slug,
        Some(authored_display_name),
    )
    .await?;

    let (existing_entity, target, detail_id, write_abilities, adoption) = if let Some(adopt_id) =
        request.adopt_workflow_id
    {
        if !scan.exact_matches.is_empty() {
            return Err(AppError::Reconciliation(
                "workflow adoption requires zero existing exact identity markers".into(),
            ));
        }
        let detail = fetch_detail(client, base_url, adopt_id).await?;
        if detail.id != adopt_id || detail.project != request.project_name {
            return Err(AppError::Reconciliation(
                "workflow adoption candidate does not match the selected project and id".into(),
            ));
        }
        if classify_marker(
            detail.meta_config.0.as_deref(),
            &detail.project,
            request.project_name,
            request.slug,
        ) != MarkerClassification::Unmarked
        {
            return Err(AppError::Reconciliation(
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
            },
            Some(id),
            vec![ability],
            true,
        )
    } else {
        match scan.exact_matches.as_slice() {
            [] => {
                if !scan.unmarked_display_name_ids.is_empty() {
                    return Err(AppError::Reconciliation(
                        "workflow identity requires explicit adoption".into(),
                    ));
                }
                (None, ResolutionTarget::Create, None, Vec::new(), false)
            }
            [single] => {
                let list_ability = prove_write(&single.user_abilities, "Workflow")?;
                let detail = fetch_detail(client, base_url, &single.id).await?;
                if detail.id != single.id || detail.project != request.project_name {
                    return Err(AppError::Reconciliation(
                        "workflow detail no longer matches the resolved identity".into(),
                    ));
                }
                if classify_marker(
                    detail.meta_config.0.as_deref(),
                    &detail.project,
                    request.project_name,
                    request.slug,
                ) != MarkerClassification::Exact
                {
                    return Err(AppError::Reconciliation(
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

    let references = resolve_execution_references(client, base_url, request.declaration).await?;

    let plan = project_with_workflow_references(
        request.declaration,
        existing_entity.as_ref(),
        request.adopt_workflow_id,
        request.repo_root,
        request.follow_symlinks,
        Some(&references.map),
    )?;

    let resolution = CompletedResolution {
        effective_project: request.project_name.to_owned(),
        _slug: request.slug.to_owned(),
        target,
        _scope_scans: scan.evidence,
        _resolved_references: references.map,
        _detail_id: detail_id,
        _write_abilities: write_abilities,
        _adoption: adoption,
    };
    let prepared = PreparedWrite::workflow(visibility, resolution, plan)?;
    dispatch(client, base_url, prepared).await
}

async fn dispatch(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    prepared: PreparedWrite,
) -> Result<ApplyResult, AppError> {
    let action = match prepared.target() {
        ResolutionTarget::Create => ApplyAction::Created,
        ResolutionTarget::Update { .. } => ApplyAction::Updated,
    };
    let response = client.dispatch_prepared(base_url, prepared).await?;
    let response: WorkflowIdResponse = decode_write_response(response)?.ok_or_else(|| {
        AppError::Internal("Workflow modifying request cannot return a conflict signal".into())
    })?;
    Ok(ApplyResult {
        action,
        server_id: response.id,
    })
}

/// Re-resolve a Workflow marker after a modifying request and require that it
/// identifies exactly the server route returned by that request (FR-034).
pub async fn verify_identity(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
    expected_server_id: &str,
) -> Result<(), AppError> {
    let scan = enumerate_all(client, base_url, project_name, slug, None).await?;
    match scan.exact_matches.as_slice() {
        [single] if single.id == expected_server_id => Ok(()),
        _ => Err(AppError::Reconciliation(
            "Workflow write may have committed but identity verification did not match exactly once"
                .into(),
        )),
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
    base_url: &ValidatedUrl,
    decl: &ParsedDeclaration,
) -> Result<ResolvedWorkflowReferences, AppError> {
    let keys = collect_execution_reference_keys(decl)?;
    let mut resolved = WorkflowReferenceMap::default();

    for (project, slug) in keys.assistants {
        let server_id = assistant::resolve_reference(client, base_url, &project, &slug).await?;
        resolved.insert_assistant(project, slug, server_id);
    }
    for (project, name) in keys.skills {
        let server_id = skill::resolve_reference(client, base_url, &project, &name).await?;
        resolved.insert_skill(project, name, server_id);
    }
    for (project, repo_name) in keys.datasources {
        let server_id =
            datasource::resolve_reference(client, base_url, &project, &repo_name).await?;
        resolved.insert_datasource(project, repo_name, server_id);
    }

    Ok(ResolvedWorkflowReferences { map: resolved })
}

fn collect_execution_reference_keys(
    decl: &ParsedDeclaration,
) -> Result<WorkflowReferenceKeys, AppError> {
    let actors = decl
        .value
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
            continue;
        }

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
    Ok(keys)
}

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

async fn enumerate_all(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    project_name: &str,
    slug: &str,
    authored_display_name: Option<&str>,
) -> Result<WorkflowScan, AppError> {
    let mut all_matches = Vec::new();
    let mut all_unmarked_display_names = Vec::new();
    let mut seen_ids = std::collections::HashSet::new();
    let mut seen_unmarked_ids = std::collections::HashSet::new();
    let mut evidence = Vec::with_capacity(2);

    // Pass 1: project-visible (no scope parameter)
    // Pass 2: globally published (scope=marketplace)
    for scope in [None, Some("marketplace")] {
        let pass = enumerate_pass(
            client,
            base_url,
            scope,
            project_name,
            slug,
            authored_display_name,
        )
        .await?;
        for item in pass.exact_matches {
            if seen_ids.insert(item.id.clone()) {
                all_matches.push(item);
            }
        }
        for id in pass.unmarked_display_name_ids {
            if seen_unmarked_ids.insert(id.clone()) {
                all_unmarked_display_names.push(id);
            }
        }
        evidence.push(pass.evidence);
    }

    Ok(WorkflowScan {
        exact_matches: all_matches,
        unmarked_display_name_ids: all_unmarked_display_names,
        evidence,
    })
}

async fn enumerate_pass(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    scope: Option<&'static str>,
    project_name: &str,
    slug: &str,
    authored_display_name: Option<&str>,
) -> Result<PassResult, AppError> {
    let mut matches = Vec::new();
    let mut unmarked_display_name_ids = Vec::new();
    let mut page = 0u32;
    let mut pages_requested = 0u32;
    let mut total_seen = 0u32;
    let mut fingerprint: Option<(u32, u32, u32)> = None;
    let mut seen_ids = std::collections::HashSet::new();

    loop {
        let path = match scope {
            None => format!(
                "/v1/workflows?minimal_response=false&page={}&per_page=100",
                page
            ),
            Some(s) => format!(
                "/v1/workflows?minimal_response=false&page={}&per_page=100&scope={}",
                page,
                encode_query_value(s)
            ),
        };

        let resp: WorkflowPage = client.get(base_url, &path).await?;
        pages_requested += 1;
        validate_pagination(page, &resp.pagination, fingerprint)?;
        fingerprint = Some((
            resp.pagination.pages,
            resp.pagination.total,
            resp.pagination.per_page,
        ));
        let total_pages = resp.pagination.pages;

        for item in resp.data {
            total_seen += 1;
            if total_seen > MAX_ITEMS {
                return Err(AppError::ApiIncompatible(
                    "workflow enumeration exceeded 100,000-item cap".into(),
                ));
            }
            if !seen_ids.insert(item.id.clone()) {
                return Err(AppError::Reconciliation(
                    "workflow enumeration repeated an entity id".into(),
                ));
            }
            if item.project == project_name {
                match classify_marker(
                    item.meta_config.0.as_deref(),
                    &item.project,
                    project_name,
                    slug,
                ) {
                    MarkerClassification::Exact => matches.push(item),
                    MarkerClassification::Unmarked
                        if authored_display_name.is_some_and(|name| item.name == name) =>
                    {
                        unmarked_display_name_ids.push(item.id)
                    }
                    MarkerClassification::Invalid => {
                        return Err(AppError::Reconciliation(
                            "workflow target project contains invalid identity metadata".into(),
                        ));
                    }
                    MarkerClassification::Unmarked | MarkerClassification::OtherValid => {}
                }
            }
        }

        if total_pages == 0 || page + 1 >= total_pages {
            break;
        }
        page += 1;
    }

    let expected_total = fingerprint.map_or(0, |(_, total, _)| total);
    if total_seen != expected_total {
        return Err(AppError::Reconciliation(
            "workflow enumeration ended before the advertised total".into(),
        ));
    }

    Ok(PassResult {
        exact_matches: matches,
        unmarked_display_name_ids,
        evidence: PassEvidence {
            _scope: scope,
            _pages_requested: pages_requested,
            _items_seen: total_seen,
            _advertised_total: expected_total,
        },
    })
}

fn validate_pagination(
    requested_page: u32,
    pagination: &WorkflowPagination,
    fingerprint: Option<(u32, u32, u32)>,
) -> Result<(), AppError> {
    if pagination.page != requested_page || pagination.per_page != 100 {
        return Err(AppError::ApiIncompatible(
            "workflow pagination origin or page size changed".into(),
        ));
    }
    if pagination.pages > MAX_PAGES {
        return Err(AppError::ApiIncompatible(
            "workflow enumeration exceeded 1,000-page cap".into(),
        ));
    }
    if (pagination.pages == 0) != (pagination.total == 0) {
        return Err(AppError::ApiIncompatible(
            "workflow zero page count does not match zero total".into(),
        ));
    }
    let computed_pages = pagination.total.div_ceil(pagination.per_page);
    if pagination.pages != computed_pages {
        return Err(AppError::ApiIncompatible(
            "workflow pagination total and page count disagree".into(),
        ));
    }
    if let Some(expected) = fingerprint
        && expected != (pagination.pages, pagination.total, pagination.per_page)
    {
        return Err(AppError::Reconciliation(
            "workflow pagination changed during enumeration".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Explicit adoption: fetch current state for meta_config merge
// ---------------------------------------------------------------------------

async fn fetch_detail(
    client: &ApiClient,
    base_url: &ValidatedUrl,
    workflow_id: &str,
) -> Result<WorkflowItem, AppError> {
    let path = format!("/v1/workflows/id/{}", encode_query_value(workflow_id));
    client.get(base_url, &path).await
}

// ---------------------------------------------------------------------------
// Identity classification: strict JSON and closed reserved record (ADR-008)
// ---------------------------------------------------------------------------

fn classify_marker(
    meta_config: Option<&str>,
    row_project: &str,
    desired_project: &str,
    desired_slug: &str,
) -> MarkerClassification {
    let Some(raw) = meta_config else {
        return MarkerClassification::Unmarked;
    };
    let Ok(value) = parse_strict_json(raw) else {
        return MarkerClassification::Invalid;
    };
    let Some(object) = value.as_object() else {
        return MarkerClassification::Invalid;
    };
    let Some(identity) = object.get(IDENTITY_KEY) else {
        return MarkerClassification::Unmarked;
    };
    let Some(identity) = identity.as_object() else {
        return MarkerClassification::Invalid;
    };
    if identity.len() != 3
        || !identity.contains_key("version")
        || !identity.contains_key("project")
        || !identity.contains_key("slug")
    {
        return MarkerClassification::Invalid;
    }
    let Some(project) = identity.get("project").and_then(serde_json::Value::as_str) else {
        return MarkerClassification::Invalid;
    };
    let Some(slug) = identity.get("slug").and_then(serde_json::Value::as_str) else {
        return MarkerClassification::Invalid;
    };
    if identity.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        || project.is_empty()
        || slug.is_empty()
        || project != row_project
    {
        return MarkerClassification::Invalid;
    }
    if project == desired_project && slug == desired_slug {
        MarkerClassification::Exact
    } else {
        MarkerClassification::OtherValid
    }
}

/// Parse JSON while rejecting duplicate members at every nesting level.
fn parse_strict_json(raw: &str) -> Result<serde_json::Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_str(raw);
    let value = StrictValueSeed.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(value)
}

struct StrictValueSeed;

impl<'de> DeserializeSeed<'de> for StrictValueSeed {
    type Value = serde_json::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(serde_json::Value::Number)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(serde_json::Value::String(value))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        StrictValueSeed.deserialize(deserializer)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictValueSeed)? {
            values.push(value);
        }
        Ok(serde_json::Value::Array(values))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(serde::de::Error::custom("duplicate JSON object member"));
            }
            let value = map.next_value_seed(StrictValueSeed)?;
            values.insert(key, value);
        }
        Ok(serde_json::Value::Object(values))
    }
}

// ---------------------------------------------------------------------------
// Tests (W-001)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;
    use crate::parse::{EntityKind, ParsedDeclaration};

    fn test_url(base: &str) -> ValidatedUrl {
        ValidatedUrl::try_from(base).expect("test URL must be valid")
    }

    fn test_client(base: &str) -> ApiClient {
        ApiClient::new(test_url(base), "test-token".into())
            .expect("ApiClient must construct in tests")
    }

    fn user_ok_mock(
        server: &mut mockito::Server,
    ) -> impl std::future::Future<Output = mockito::Mock> + '_ {
        server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"my-project","is_project_admin":true}]}"#,
            )
            .create_async()
    }

    fn workflow_decl(project: &str, slug: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: json!({
                "metadata": { "project": project, "slug": slug },
                "spec": {
                    "name": "Test Workflow",
                    "description": "A test workflow",
                    "mode": "chat",
                    "shared": false,
                    "execution_config": {
                        "assistants": [],
                        "states": [],
                        "tools": []
                    }
                }
            }),
            source_path: PathBuf::from("test.yaml"),
        }
    }

    fn meta_config_for(project: &str, slug: &str) -> String {
        format!(r#"{{"{IDENTITY_KEY}":{{"version":1,"project":"{project}","slug":"{slug}"}}}}"#)
    }

    fn empty_page() -> &'static str {
        r#"{"data":[],"pagination":{"page":0,"per_page":100,"total":0,"pages":0}}"#
    }

    fn one_match_page(id: &str, project: &str, slug: &str) -> String {
        let mc = meta_config_for(project, slug);
        format!(
            r#"{{"data":[{{"id":"{id}","project":"{project}","name":"Workflow","meta_config":{mc_json},"user_abilities":["read","write"]}}],"pagination":{{"page":0,"per_page":100,"total":1,"pages":1}}}}"#,
            mc_json = serde_json::to_string(&mc).unwrap()
        )
    }

    fn two_match_page(project: &str, slug: &str) -> String {
        let mc = meta_config_for(project, slug);
        let mc_json = serde_json::to_string(&mc).unwrap();
        format!(
            r#"{{"data":[{{"id":"id-1","project":"{project}","name":"Workflow 1","meta_config":{mc_json},"user_abilities":["write"]}},{{"id":"id-2","project":"{project}","name":"Workflow 2","meta_config":{mc_json},"user_abilities":["write"]}}],"pagination":{{"page":0,"per_page":100,"total":2,"pages":1}}}}"#
        )
    }

    // -----------------------------------------------------------------------
    // W-001 AC: zero matches → POST/created
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_creates_when_not_found() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        // Both passes return empty
        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;

        let no_page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100.*$".to_owned(),
                ),
            )
            .expect(0)
            .create_async()
            .await;

        let _create = server
            .mock("POST", "/v1/workflows")
            .with_status(201)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"wf-new"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Created);
        assert_eq!(result.server_id, "wf-new");
        _p1.assert_async().await;
        _p2.assert_async().await;
        no_page_one.assert_async().await;
        _create.assert_async().await;
    }

    // -----------------------------------------------------------------------
    // W-001 AC: one match → PUT/updated
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_updates_when_found() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("wf-exist", "my-project", "my-slug"))
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let _detail = server
            .mock("GET", "/v1/workflows/id/wf-exist")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(format!(
                r#"{{"id":"wf-exist","project":"my-project","name":"Workflow","meta_config":{},"user_abilities":["read","write"]}}"#,
                serde_json::to_string(&meta_config_for("my-project", "my-slug")).unwrap()
            ))
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/workflows/wf-exist")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"wf-exist"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect("apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "wf-exist");
        _p1.assert_async().await;
        _p2.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_other_project_admin_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let user = server
            .mock("GET", "/v1/user")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"is_admin":false,"is_maintainer":false,"projects":[{"name":"other-project","is_project_admin":true}]}"#,
            )
            .expect(1)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &workflow_decl("my-project", "my-slug"),
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("another project's admin entry is insufficient");
        assert!(matches!(error, AppError::VisibilityUnproven(_)));
        user.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_nonwriting_row_before_put() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let page = one_match_page("wf-exist", "my-project", "my-slug")
            .replace(r#"["read","write"]"#, r#"["read"]"#);
        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page)
            .create_async()
            .await;
        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &workflow_decl("my-project", "my-slug"),
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("non-writing Workflow row must stop before PUT");
        assert!(matches!(error, AppError::Authorization(_)));
        put.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_pagination_drift_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let page = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[],"pagination":{"page":1,"per_page":100,"total":0,"pages":0}}"#)
            .expect(1)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &workflow_decl("my-project", "my-slug"),
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("wrong returned page must fail compatibility");
        assert!(matches!(error, AppError::ApiIncompatible(_)));
        page.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[tokio::test]
    async fn apply_rejects_repeated_workflow_ids_before_any_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let repeated = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"data":[{"id":"same","project":"my-project","name":"Other A","meta_config":null,"user_abilities":["read"]},{"id":"same","project":"my-project","name":"Other B","meta_config":null,"user_abilities":["read"]}],"pagination":{"page":0,"per_page":100,"total":2,"pages":1}}"#,
            )
            .expect(1)
            .create_async()
            .await;
        let post = server
            .mock("POST", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &workflow_decl("my-project", "my-slug"),
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("repeated IDs indicate snapshot instability");

        assert!(matches!(error, AppError::Reconciliation(_)));
        repeated.assert_async().await;
        post.assert_async().await;
        put.assert_async().await;
    }

    #[test]
    fn workflow_response_requires_consumed_fields_but_allows_null_meta_and_additions() {
        let baseline = json!({
            "data": [{
                "id": "wf",
                "project": "my-project",
                "name": "Workflow",
                "meta_config": null,
                "user_abilities": ["write"]
            }],
            "pagination": {"page": 0, "pages": 1, "total": 1, "per_page": 100}
        });
        let additive = json!({
            "data": [{
                "id": "wf",
                "project": "my-project",
                "name": "Workflow",
                "meta_config": null,
                "user_abilities": ["write"],
                "future_entity": true
            }],
            "pagination": {"page": 0, "pages": 1, "total": 1, "per_page": 100, "future_page": 7},
            "future_top": {}
        });
        let baseline_page =
            serde_json::from_value::<WorkflowPage>(baseline.clone()).expect("baseline must decode");
        let additive_page =
            serde_json::from_value::<WorkflowPage>(additive).expect("additions must decode");

        crate::adapters::assert_consumed_field_mutations::<WorkflowPage>(
            &baseline,
            &[
                "/data",
                "/data/0/id",
                "/data/0/project",
                "/data/0/name",
                "/data/0/meta_config",
                "/data/0/user_abilities",
                "/pagination",
                "/pagination/page",
                "/pagination/pages",
                "/pagination/total",
                "/pagination/per_page",
            ],
        );

        let project_response = |page: WorkflowPage| {
            let item = &page.data[0];
            let existing = ExistingEntity {
                server_id: item.id.clone(),
                meta_config: item.meta_config.0.clone(),
            };
            project_with_workflow_references(
                &workflow_decl("my-project", "my-slug"),
                Some(&existing),
                None,
                Path::new("."),
                false,
                Some(&WorkflowReferenceMap::default()),
            )
            .expect("decoded response must project")
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(baseline_body),
            ..
        } = project_response(baseline_page)
        else {
            panic!("Workflow projection must be JSON");
        };
        let crate::projection::WritePlan::Update {
            request: crate::projection::RequestBody::Json(additive_body),
            ..
        } = project_response(additive_page)
        else {
            panic!("Workflow projection must be JSON");
        };
        assert_eq!(baseline_body, additive_body);
    }

    #[test]
    fn workflow_pagination_invariant_matrix() {
        let empty = || WorkflowPagination {
            page: 0,
            pages: 0,
            total: 0,
            per_page: 100,
        };
        assert!(validate_pagination(0, &empty(), None).is_ok());

        for invalid in [
            WorkflowPagination { page: 1, ..empty() },
            WorkflowPagination {
                per_page: 99,
                ..empty()
            },
            WorkflowPagination {
                total: 1,
                pages: 0,
                ..empty()
            },
            WorkflowPagination {
                total: 101,
                pages: 1,
                ..empty()
            },
            WorkflowPagination {
                total: 100_001,
                pages: 1_001,
                ..empty()
            },
        ] {
            assert!(matches!(
                validate_pagination(0, &invalid, None),
                Err(AppError::ApiIncompatible(_))
            ));
        }

        let stable_page = WorkflowPagination {
            page: 1,
            pages: 2,
            total: 101,
            per_page: 100,
        };
        assert!(validate_pagination(1, &stable_page, Some((2, 101, 100))).is_ok());
        assert!(matches!(
            validate_pagination(1, &stable_page, Some((2, 100, 100))),
            Err(AppError::Reconciliation(_))
        ));
    }

    #[tokio::test]
    async fn verify_identity_traverses_zero_based_workflow_pages_and_both_scopes() {
        let mut server = mockito::Server::new_async().await;
        let first_page_items = (0..100)
            .map(|index| {
                json!({
                    "id": format!("other-{index}"),
                    "project": "other-project",
                    "name": "Other Workflow",
                    "meta_config": null,
                    "user_abilities": ["read"]
                })
            })
            .collect::<Vec<_>>();
        let page_zero = json!({
            "data": first_page_items,
            "pagination": {"page": 0, "per_page": 100, "total": 101, "pages": 2}
        });
        let page_one = json!({
            "data": [{
                "id": "expected-workflow",
                "project": "my-project",
                "name": "Workflow",
                "meta_config": meta_config_for("my-project", "my-slug"),
                "user_abilities": ["write"]
            }],
            "pagination": {"page": 1, "per_page": 100, "total": 101, "pages": 2}
        });
        let project_page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page_zero.to_string())
            .expect(1)
            .create_async()
            .await;
        let project_page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(page_one.to_string())
            .expect(1)
            .create_async()
            .await;
        let marketplace_page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                        .to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let marketplace_page_one = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=1&per_page=100&scope=marketplace$"
                        .to_owned(),
                ),
            )
            .expect(0)
            .create_async()
            .await;

        verify_identity(
            &test_client(&server.url()),
            &test_url(&server.url()),
            "my-project",
            "my-slug",
            "expected-workflow",
        )
        .await
        .expect("post-write scanner must traverse pages 0 and 1");

        project_page_zero.assert_async().await;
        project_page_one.assert_async().await;
        marketplace_page_zero.assert_async().await;
        marketplace_page_one.assert_async().await;
    }

    #[tokio::test]
    async fn verify_identity_rejects_nonzero_origin_without_modification() {
        let mut server = mockito::Server::new_async().await;
        let page_zero = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"data":[],"pagination":{"page":1,"per_page":100,"total":0,"pages":0}}"#)
            .expect(1)
            .create_async()
            .await;
        let marketplace = server
            .mock(
                "GET",
                mockito::Matcher::Regex(r"^/v1/workflows\?.*scope=marketplace$".to_owned()),
            )
            .expect(0)
            .create_async()
            .await;
        let mut modifications = Vec::new();
        for method in ["POST", "PUT", "PATCH", "DELETE"] {
            modifications.push(
                server
                    .mock(method, mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await,
            );
        }

        let error = verify_identity(
            &test_client(&server.url()),
            &test_url(&server.url()),
            "my-project",
            "my-slug",
            "expected-workflow",
        )
        .await
        .expect_err("invalid post-write page origin must fail closed");

        assert!(matches!(error, AppError::ApiIncompatible(_)));
        page_zero.assert_async().await;
        marketplace.assert_async().await;
        for modification in modifications {
            modification.assert_async().await;
        }
    }

    // -----------------------------------------------------------------------
    // W-001 AC: multiple matches → Reconciliation error
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_reconciliation_on_multiple_matches() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(two_match_page("my-project", "my-slug"))
            .create_async()
            .await;

        let _p2 = server
            .mock("GET", mockito::Matcher::Regex(
                r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$".to_string()
            ))
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let err = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: None,
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("multiple matches must error");

        assert!(matches!(err, AppError::Reconciliation(_)));
        assert_eq!(err.exit_code(), 1);
    }

    // -----------------------------------------------------------------------
    // W-001 AC: adoption exhausts both scopes, then validates an unmarked row
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn apply_adopt_id_requires_zero_marker_scan_and_unmarked_candidate() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;

        let _p1 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let _p2 = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$".to_string(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;

        let _detail = server
            .mock("GET", "/v1/workflows/id/adopt-wf-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"id":"adopt-wf-id","project":"my-project","name":"Workflow","meta_config":"{\"other\":true}","user_abilities":["write"]}"#,
            )
            .create_async()
            .await;

        let _update = server
            .mock("PUT", "/v1/workflows/adopt-wf-id")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"id":"adopt-wf-id"}"#)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let decl = workflow_decl("my-project", "my-slug");

        let result = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &decl,
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: Some("adopt-wf-id"),
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect("adopt-id apply must succeed");

        assert_eq!(result.action, ApplyAction::Updated);
        assert_eq!(result.server_id, "adopt-wf-id");
        _detail.assert_async().await;
        _update.assert_async().await;
    }

    #[tokio::test]
    async fn invalid_target_project_markers_in_either_scope_fail_closed_with_no_modification() {
        let invalid_markers = [
            "not-json",
            "[]",
            r#"{"codemie.epam.com/gitops/workflow-identity":{"version":"1","project":"my-project","slug":"my-slug"}}"#,
            r#"{"codemie.epam.com/gitops/workflow-identity":{"version":1,"project":"my-project","slug":"my-slug"},"codemie.epam.com/gitops/workflow-identity":{"version":1,"project":"my-project","slug":"my-slug"}}"#,
        ];

        for marketplace_scope in [false, true] {
            for (case_index, marker) in invalid_markers.iter().enumerate() {
                let mut server = mockito::Server::new_async().await;
                let _user = user_ok_mock(&mut server).await;
                let invalid_page = format!(
                    r#"{{"data":[{{"id":"wf-invalid-{case_index}","project":"my-project","name":"Workflow","meta_config":{},"user_abilities":["write"]}}],"pagination":{{"page":0,"per_page":100,"total":1,"pages":1}}}}"#,
                    serde_json::to_string(marker).unwrap()
                );
                let project_body = if marketplace_scope {
                    empty_page().to_owned()
                } else {
                    invalid_page.clone()
                };
                let marketplace_body = if marketplace_scope {
                    invalid_page
                } else {
                    empty_page().to_owned()
                };
                let project_scan = server
                    .mock(
                        "GET",
                        mockito::Matcher::Regex(
                            r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$"
                                .to_owned(),
                        ),
                    )
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_body(project_body)
                    .expect(1)
                    .create_async()
                    .await;
                let marketplace_scan = server
                    .mock(
                        "GET",
                        mockito::Matcher::Regex(
                            r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                                .to_owned(),
                        ),
                    )
                    .with_status(200)
                    .with_header("content-type", "application/json")
                    .with_body(marketplace_body)
                    .expect(usize::from(marketplace_scope))
                    .create_async()
                    .await;
                let post = server
                    .mock("POST", mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;
                let put = server
                    .mock("PUT", mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;
                let patch = server
                    .mock("PATCH", mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;
                let delete = server
                    .mock("DELETE", mockito::Matcher::Any)
                    .expect(0)
                    .create_async()
                    .await;

                let url = test_url(&server.url());
                let client = test_client(&server.url());
                let error = apply(
                    &client,
                    &url,
                    ApplyRequest {
                        declaration: &workflow_decl("my-project", "my-slug"),
                        project_name: "my-project",
                        slug: "my-slug",
                        adopt_workflow_id: None,
                        repo_root: Path::new("."),
                        follow_symlinks: false,
                    },
                )
                .await
                .expect_err("invalid marker must fail before every modifying route");

                assert!(matches!(error, AppError::Reconciliation(_)));
                project_scan.assert_async().await;
                marketplace_scan.assert_async().await;
                post.assert_async().await;
                put.assert_async().await;
                patch.assert_async().await;
                delete.assert_async().await;
            }
        }
    }

    #[tokio::test]
    async fn adoption_candidate_fault_matrix_stops_before_every_modifying_route() {
        let exact = meta_config_for("my-project", "other-slug");
        let cases = [
            ("my-project", Some(exact.as_str()), vec!["write"]),
            ("my-project", Some("not-json"), vec!["write"]),
            ("other-project", Some(r#"{"other":true}"#), vec!["write"]),
            ("my-project", Some(r#"{"other":true}"#), vec!["read"]),
        ];

        for (case_index, (project, meta_config, abilities)) in cases.iter().enumerate() {
            let mut server = mockito::Server::new_async().await;
            let _user = user_ok_mock(&mut server).await;
            let _p1 = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(empty_page())
                .expect(1)
                .create_async()
                .await;
            let _p2 = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                            .to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(empty_page())
                .expect(1)
                .create_async()
                .await;
            let detail_body = serde_json::json!({
                "id": format!("candidate-{case_index}"),
                "project": project,
                "name": "Workflow",
                "meta_config": meta_config,
                "user_abilities": abilities,
            });
            let detail = server
                .mock(
                    "GET",
                    format!("/v1/workflows/id/candidate-{case_index}").as_str(),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(detail_body.to_string())
                .expect(1)
                .create_async()
                .await;
            let post = server
                .mock("POST", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let put = server
                .mock("PUT", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let patch = server
                .mock("PATCH", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let delete = server
                .mock("DELETE", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;

            let url = test_url(&server.url());
            let client = test_client(&server.url());
            let selector = format!("candidate-{case_index}");
            let error = apply(
                &client,
                &url,
                ApplyRequest {
                    declaration: &workflow_decl("my-project", "my-slug"),
                    project_name: "my-project",
                    slug: "my-slug",
                    adopt_workflow_id: Some(&selector),
                    repo_root: Path::new("."),
                    follow_symlinks: false,
                },
            )
            .await
            .expect_err("unsafe adoption candidate must not reach modification");

            assert!(matches!(
                error,
                AppError::Reconciliation(_) | AppError::Authorization(_)
            ));
            detail.assert_async().await;
            post.assert_async().await;
            put.assert_async().await;
            patch.assert_async().await;
            delete.assert_async().await;
        }
    }

    #[tokio::test]
    async fn adoption_existing_exact_marker_stops_before_candidate_read_and_write() {
        let mut server = mockito::Server::new_async().await;
        let _user = user_ok_mock(&mut server).await;
        let project_scan = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(one_match_page("existing", "my-project", "my-slug"))
            .expect(1)
            .create_async()
            .await;
        let marketplace_scan = server
            .mock(
                "GET",
                mockito::Matcher::Regex(
                    r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                        .to_owned(),
                ),
            )
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(empty_page())
            .expect(1)
            .create_async()
            .await;
        let candidate = server
            .mock("GET", "/v1/workflows/id/candidate")
            .expect(0)
            .create_async()
            .await;
        let put = server
            .mock("PUT", mockito::Matcher::Any)
            .expect(0)
            .create_async()
            .await;

        let url = test_url(&server.url());
        let client = test_client(&server.url());
        let error = apply(
            &client,
            &url,
            ApplyRequest {
                declaration: &workflow_decl("my-project", "my-slug"),
                project_name: "my-project",
                slug: "my-slug",
                adopt_workflow_id: Some("candidate"),
                repo_root: Path::new("."),
                follow_symlinks: false,
            },
        )
        .await
        .expect_err("an existing marker must veto adoption before candidate read");

        assert!(matches!(error, AppError::Reconciliation(_)));
        project_scan.assert_async().await;
        marketplace_scan.assert_async().await;
        candidate.assert_async().await;
        put.assert_async().await;
    }

    #[tokio::test]
    async fn unmarked_same_display_name_row_does_not_veto_explicit_adoption() {
        let unmarked_page = r#"{"data":[{"id":"other-legacy","project":"my-project","name":"Test Workflow","meta_config":null,"user_abilities":["write"]}],"pagination":{"page":0,"per_page":100,"total":1,"pages":1}}"#;

        for marketplace_scope in [false, true] {
            let mut server = mockito::Server::new_async().await;
            let _user = user_ok_mock(&mut server).await;
            let project_scan = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(if marketplace_scope {
                    empty_page()
                } else {
                    unmarked_page
                })
                .expect(1)
                .create_async()
                .await;
            let marketplace_scan = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                            .to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(if marketplace_scope {
                    unmarked_page
                } else {
                    empty_page()
                })
                .expect(1)
                .create_async()
                .await;
            let candidate = server
                .mock("GET", "/v1/workflows/id/candidate")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(
                    r#"{"id":"candidate","project":"my-project","name":"Candidate","meta_config":null,"user_abilities":["write"]}"#,
                )
                .expect(1)
                .create_async()
                .await;
            let post = server
                .mock("POST", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let put = server
                .mock("PUT", "/v1/workflows/candidate")
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(r#"{"id":"candidate"}"#)
                .expect(1)
                .create_async()
                .await;
            let patch = server
                .mock("PATCH", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let delete = server
                .mock("DELETE", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;

            let result = apply(
                &test_client(&server.url()),
                &test_url(&server.url()),
                ApplyRequest {
                    declaration: &workflow_decl("my-project", "my-slug"),
                    project_name: "my-project",
                    slug: "my-slug",
                    adopt_workflow_id: Some("candidate"),
                    repo_root: Path::new("."),
                    follow_symlinks: false,
                },
            )
            .await
            .expect("an unrelated unmarked display-name row must not veto explicit adoption");

            assert_eq!(result.action, ApplyAction::Updated);
            project_scan.assert_async().await;
            marketplace_scan.assert_async().await;
            candidate.assert_async().await;
            post.assert_async().await;
            put.assert_async().await;
            patch.assert_async().await;
            delete.assert_async().await;
        }
    }

    #[tokio::test]
    async fn unmarked_authored_display_name_blocks_create_when_name_differs_from_slug() {
        let unmarked_page = r#"{"data":[{"id":"legacy","project":"my-project","name":"Test Workflow","meta_config":null,"user_abilities":["read","write"]}],"pagination":{"page":0,"per_page":100,"total":1,"pages":1}}"#;

        for marketplace_scope in [false, true] {
            let mut server = mockito::Server::new_async().await;
            let _user = user_ok_mock(&mut server).await;
            let project_scan = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100$".to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(if marketplace_scope {
                    empty_page()
                } else {
                    unmarked_page
                })
                .expect(1)
                .create_async()
                .await;
            let marketplace_scan = server
                .mock(
                    "GET",
                    mockito::Matcher::Regex(
                        r"^/v1/workflows\?minimal_response=false&page=0&per_page=100&scope=marketplace$"
                            .to_owned(),
                    ),
                )
                .with_status(200)
                .with_header("content-type", "application/json")
                .with_body(if marketplace_scope {
                    unmarked_page
                } else {
                    empty_page()
                })
                .expect(1)
                .create_async()
                .await;
            let post = server
                .mock("POST", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let put = server
                .mock("PUT", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let patch = server
                .mock("PATCH", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;
            let delete = server
                .mock("DELETE", mockito::Matcher::Any)
                .expect(0)
                .create_async()
                .await;

            let url = test_url(&server.url());
            let client = test_client(&server.url());
            let error = apply(
                &client,
                &url,
                ApplyRequest {
                    declaration: &workflow_decl("my-project", "my-slug"),
                    project_name: "my-project",
                    slug: "my-slug",
                    adopt_workflow_id: None,
                    repo_root: Path::new("."),
                    follow_symlinks: false,
                },
            )
            .await
            .expect_err("authored display-name match must require explicit adoption");

            assert!(matches!(error, AppError::Reconciliation(_)));
            assert_eq!(error.exit_code(), 1);
            project_scan.assert_async().await;
            marketplace_scan.assert_async().await;
            post.assert_async().await;
            put.assert_async().await;
            patch.assert_async().await;
            delete.assert_async().await;
        }
    }

    // -----------------------------------------------------------------------
    // Strict identity classification helpers
    // -----------------------------------------------------------------------

    #[test]
    fn marker_classification_exact() {
        let mc = meta_config_for("proj", "sl");
        assert_eq!(
            classify_marker(Some(&mc), "proj", "proj", "sl"),
            MarkerClassification::Exact
        );
    }

    #[test]
    fn marker_classification_conflicting_row_project_is_invalid() {
        let mc = meta_config_for("other", "sl");
        assert_eq!(
            classify_marker(Some(&mc), "proj", "proj", "sl"),
            MarkerClassification::Invalid
        );
    }

    #[test]
    fn marker_classification_other_valid_slug() {
        let mc = meta_config_for("proj", "other");
        assert_eq!(
            classify_marker(Some(&mc), "proj", "proj", "sl"),
            MarkerClassification::OtherValid
        );
    }

    #[test]
    fn marker_classification_missing_reserved_key_is_unmarked() {
        assert_eq!(
            classify_marker(Some(r#"{"other": "value"}"#), "proj", "proj", "sl"),
            MarkerClassification::Unmarked
        );
    }

    #[test]
    fn marker_classification_invalid_shapes_fail_closed() {
        for raw in [
            "not-json",
            "[]",
            r#"{"codemie.epam.com/gitops/workflow-identity":null}"#,
            r#"{"codemie.epam.com/gitops/workflow-identity":{"version":"1","project":"proj","slug":"sl"}}"#,
            r#"{"codemie.epam.com/gitops/workflow-identity":{"version":1,"project":"proj","slug":"sl","extra":true}}"#,
            r#"{"codemie.epam.com/gitops/workflow-identity":{"version":1,"project":"proj","slug":"sl"},"codemie.epam.com/gitops/workflow-identity":{"version":1,"project":"proj","slug":"sl"}}"#,
        ] {
            assert_eq!(
                classify_marker(Some(raw), "proj", "proj", "sl"),
                MarkerClassification::Invalid,
                "raw={raw}"
            );
        }
    }

    #[test]
    fn marker_classification_null_container_is_unmarked() {
        assert_eq!(
            classify_marker(None, "proj", "proj", "sl"),
            MarkerClassification::Unmarked
        );
    }
}
