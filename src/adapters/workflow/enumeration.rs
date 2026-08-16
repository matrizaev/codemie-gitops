//! Workflow enumeration and reserved-marker classification.
//!
//! Two-pass zero-based enumeration (project-visible then scope=marketplace),
//! deduplicated across passes by server ID, plus the closed reserved
//! workflow-identity marker codec used for reconciliation, adoption, and
//! snapshot selection.

use serde::Deserialize;

use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};

pub(crate) const IDENTITY_KEY: &str = "codemie.epam.com/gitops/workflow-identity";

#[derive(Deserialize)]
pub(super) struct WorkflowPage {
    data: Vec<WorkflowItem>,
    pagination: WorkflowPagination,
}

#[derive(Deserialize)]
pub(super) struct WorkflowPagination {
    page: u32,
    pages: u32,
    total: u32,
    per_page: u32,
}

#[derive(Deserialize, Clone)]
pub(super) struct WorkflowItem {
    pub(super) id: String,
    pub(super) project: String,
    #[serde(rename = "name")]
    pub(super) name: String,
    pub(super) meta_config: RequiredNullableString,
    pub(super) created_by: Creator,
    pub(super) user_abilities: Vec<String>,
    #[serde(default)]
    pub(super) mode: Option<String>,
    #[serde(default)]
    pub(super) deprecated: bool,
}

#[derive(Deserialize, Clone)]
pub(super) struct Creator {
    #[serde(alias = "user_id")]
    pub(super) id: String,
}
#[derive(Clone)]
pub(super) struct RequiredNullableString(pub(super) Option<String>);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MarkerClassification {
    Unmarked,
    Exact,
    OtherValid,
    Invalid,
}

#[derive(Debug)]
pub(super) struct PassEvidence {
    _scope: Option<&'static str>,
    _pages_requested: u32,
    _items_seen: u32,
    _advertised_total: u32,
}

pub(super) struct PassResult {
    exact_matches: Vec<WorkflowItem>,
    unmarked_display_name_ids: Vec<String>,
    evidence: PassEvidence,
}

pub(super) struct WorkflowScan {
    pub(super) exact_matches: Vec<WorkflowItem>,
    pub(super) unmarked_display_name_ids: Vec<String>,
    pub(super) evidence: Vec<PassEvidence>,
}
pub(super) async fn enumerate_all(
    client: &ApiClient,
    project_name: &str,
    slug: &str,
    creator_user_id: &str,
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
            scope,
            project_name,
            slug,
            creator_user_id,
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
    scope: Option<&'static str>,
    project_name: &str,
    slug: &str,
    creator_user_id: &str,
    authored_display_name: Option<&str>,
) -> Result<PassResult, AppError> {
    let scan = crate::pagination::scan_pages("workflow", |page| {
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
        async move {
            let resp: WorkflowPage = client.get(&path).await?;
            for item in &resp.data {
                if item.created_by.id.is_empty() {
                    return Err(AppError::ApiIncompatible(
                        "workflow creator id is empty".into(),
                    ));
                }
            }
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

    let mut matches = Vec::new();
    let mut unmarked_display_name_ids = Vec::new();
    for item in scan.items {
        if item.project != project_name || item.created_by.id != creator_user_id {
            continue;
        }
        match classify_marker(
            item.meta_config.0.as_deref(),
            &item.project,
            project_name,
            slug,
            creator_user_id,
        ) {
            MarkerClassification::Exact => matches.push(item),
            MarkerClassification::Unmarked
                if authored_display_name.is_some_and(|name| item.name == name) =>
            {
                unmarked_display_name_ids.push(item.id)
            }
            MarkerClassification::Invalid => {
                return Err(AppError::IdentityMarkerInvalid(
                    "workflow target project contains invalid identity metadata".into(),
                ));
            }
            MarkerClassification::Unmarked | MarkerClassification::OtherValid => {}
        }
    }

    Ok(PassResult {
        exact_matches: matches,
        unmarked_display_name_ids,
        evidence: PassEvidence {
            _scope: scope,
            _pages_requested: scan.pages_requested,
            _items_seen: scan.items_seen,
            _advertised_total: scan.advertised_total,
        },
    })
}

impl crate::pagination::PageItem for WorkflowItem {
    fn page_item_id(&self) -> &str {
        &self.id
    }
}

pub(crate) fn classify_marker(
    meta_config: Option<&str>,
    row_project: &str,
    desired_project: &str,
    desired_slug: &str,
    creator_user_id: &str,
) -> MarkerClassification {
    let Some(raw) = meta_config else {
        return MarkerClassification::Unmarked;
    };
    let Ok(value) = crate::strict_json::from_str(raw) else {
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
    if identity.len() != 4
        || !identity.contains_key("version")
        || !identity.contains_key("project")
        || !identity.contains_key("slug")
        || !identity.contains_key("creator_user_id")
    {
        return MarkerClassification::Invalid;
    }
    let Some(project) = identity.get("project").and_then(serde_json::Value::as_str) else {
        return MarkerClassification::Invalid;
    };
    let Some(slug) = identity.get("slug").and_then(serde_json::Value::as_str) else {
        return MarkerClassification::Invalid;
    };
    let Some(creator) = identity
        .get("creator_user_id")
        .and_then(serde_json::Value::as_str)
    else {
        return MarkerClassification::Invalid;
    };
    if identity.get("version").and_then(serde_json::Value::as_u64) != Some(2)
        || project.is_empty()
        || slug.is_empty()
        || project != row_project
        || creator.is_empty()
    {
        return MarkerClassification::Invalid;
    }
    if project == desired_project && slug == desired_slug && creator == creator_user_id {
        MarkerClassification::Exact
    } else {
        MarkerClassification::OtherValid
    }
}
