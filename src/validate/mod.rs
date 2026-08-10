/// Offline natural and graph reference validation (F-005).
///
/// # Natural validation (`validate_natural`)
///
/// Validates per-declaration semantic invariants that the JSON Schema cannot
/// express:
///
/// - **Workflow**: `execution_config` actor IDs (`assistants[*].id`,
///   `tools[*].id`, `custom_nodes[*].id`) must be unique within their
///   respective arrays (FR-035 AC: actor uniqueness).
///   `states[*].assistant_id`, `states[*].custom_node_id`, and
///   `states[*].tool_id` must each resolve to an entry in the corresponding
///   `execution_config` array (FR-035 AC: state-local reference integrity).
///
/// No network access occurs (contracts/cli.md §4).
///
/// # Graph validation (`validate_graph`)
///
/// Validates cross-entity references across the full declaration set:
///
/// - **Duplicate natural keys** `(project, kind, key)` are rejected with
///   `AppError::Schema` exit 2 (FR-004 AC: duplicate detection).
/// - **Unresolved references**: every `assistantRef`, `skillRefs[]`,
///   `datasourceRefs[]` in Workflow actors, and every Assistant `context[].ref`,
///   `sub_assistants[]`, `skills[]` reference must resolve to a known entity in
///   the declaration set (FR-004/DR-003 AC: missing/wrong-kind/cross-project).
///
/// All violations are reported as `AppError::Schema` (exit 2).
/// No network access occurs (contracts/cli.md §4).
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::parse::{EntityKind, ParsedDeclaration};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate a single parsed declaration for semantic rules the JSON Schema
/// cannot enforce.
///
/// Currently validates Workflow-local invariants:
/// - Actor IDs in `execution_config.assistants`, `tools`, and `custom_nodes`
///   must be unique within each array.
/// - `states[*].assistant_id` must reference an id in `execution_config.assistants`.
/// - `states[*].custom_node_id` must reference an id in `execution_config.custom_nodes`.
/// - `states[*].tool_id` must reference an id in `execution_config.tools`.
///
/// For all other kinds the schema already expresses the applicable semantic
/// rules, so this function returns `Ok(())` immediately.
///
/// Returns `AppError::Schema` (exit 2) for any violation.
pub fn validate_natural(decl: &ParsedDeclaration) -> Result<(), AppError> {
    match decl.kind {
        EntityKind::Workflow => validate_workflow_local(decl),
        // Other kinds: all semantic invariants are covered by the schema (F-004).
        _ => Ok(()),
    }
}

/// Validate cross-entity references across all parsed declarations.
///
/// Performs in two phases:
/// 1. Build a `(project, kind, key) → source_path` index, failing immediately
///    on the first duplicate natural key.
/// 2. Walk every declaration and resolve each outgoing cross-entity reference
///    against the index.
///
/// Returns `AppError::Schema` (exit 2) for any violation.
pub fn validate_graph(decls: &[ParsedDeclaration]) -> Result<(), AppError> {
    let index = build_graph_index(decls)?;
    validate_all_references(decls, &index)
}

// ---------------------------------------------------------------------------
// Workflow-local (natural) validation
// ---------------------------------------------------------------------------

fn validate_workflow_local(decl: &ParsedDeclaration) -> Result<(), AppError> {
    let exec = match decl.value.pointer("/spec/execution_config") {
        Some(v) => v,
        // Missing execution_config: the schema already catches this, so treat
        // as no-op here to avoid double-reporting.
        None => return Ok(()),
    };

    // Check uniqueness and collect actor IDs in each array.
    let actor_ids = collect_unique_ids(exec, "assistants", &decl.source_path)?;
    let tool_ids = collect_unique_ids(exec, "tools", &decl.source_path)?;
    let node_ids = collect_unique_ids(exec, "custom_nodes", &decl.source_path)?;

    // Validate state-local references.
    let states = exec
        .get("states")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    for (i, state) in states.iter().enumerate() {
        if let Some(aid) = state.get("assistant_id").and_then(|v| v.as_str()) {
            if !actor_ids.contains(aid) {
                return Err(AppError::Schema(format!(
                    "'{}': workflow state[{i}].assistant_id '{}' does not match \
                     any id in execution_config.assistants; \
                     states[].assistant_id must reference a workflow-local actor id \
                     (FR-035)",
                    decl.source_path.display(),
                    aid,
                )));
            }
        }
        if let Some(nid) = state.get("custom_node_id").and_then(|v| v.as_str()) {
            if !node_ids.contains(nid) {
                return Err(AppError::Schema(format!(
                    "'{}': workflow state[{i}].custom_node_id '{}' does not match \
                     any id in execution_config.custom_nodes",
                    decl.source_path.display(),
                    nid,
                )));
            }
        }
        if let Some(tid) = state.get("tool_id").and_then(|v| v.as_str()) {
            if !tool_ids.contains(tid) {
                return Err(AppError::Schema(format!(
                    "'{}': workflow state[{i}].tool_id '{}' does not match \
                     any id in execution_config.tools",
                    decl.source_path.display(),
                    tid,
                )));
            }
        }
    }

    Ok(())
}

/// Collect the `id` values from `exec[field]` array; fail on duplicates.
///
/// Returns the complete `HashSet<String>` of IDs for caller use.
fn collect_unique_ids(
    exec: &serde_json::Value,
    field: &str,
    source_path: &Path,
) -> Result<HashSet<String>, AppError> {
    let arr = match exec.get(field).and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Ok(HashSet::new()),
    };
    let mut seen: HashSet<String> = HashSet::new();
    for item in arr {
        if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
            if !seen.insert(id.to_owned()) {
                return Err(AppError::Schema(format!(
                    "'{}': duplicate actor id '{}' in execution_config.{field}; \
                     ids must be unique within the workflow (FR-035)",
                    source_path.display(),
                    id,
                )));
            }
        }
    }
    Ok(seen)
}

// ---------------------------------------------------------------------------
// Graph index
// ---------------------------------------------------------------------------

/// Internal kind discriminant for the graph index.
///
/// Kept separate from `parse::EntityKind` so that validate's internal types
/// are stable and `Hash`-derivable without coupling to the parse module's
/// derive list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum GraphKind {
    Assistant,
    Workflow,
    Skill,
    Datasource,
}

impl GraphKind {
    fn as_str(self) -> &'static str {
        match self {
            GraphKind::Assistant => "Assistant",
            GraphKind::Workflow => "Workflow",
            GraphKind::Skill => "Skill",
            GraphKind::Datasource => "Datasource",
        }
    }
}

impl From<&EntityKind> for GraphKind {
    fn from(ek: &EntityKind) -> Self {
        match ek {
            EntityKind::Assistant => GraphKind::Assistant,
            EntityKind::Workflow => GraphKind::Workflow,
            EntityKind::Skill => GraphKind::Skill,
            EntityKind::Datasource => GraphKind::Datasource,
        }
    }
}

/// Composite natural-key tuple for graph index lookups.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GraphKey {
    project: String,
    kind: GraphKind,
    /// The kind-specific key field value: slug (Assistant/Workflow), name
    /// (Skill), or repo_name (Datasource).
    key: String,
}

/// Extract the natural-key field value (slug / name / repo_name) for a
/// declaration.
///
/// Returns `None` when the field is absent; the schema already catches that.
fn natural_key_of(kind: &EntityKind, value: &serde_json::Value) -> Option<String> {
    let field = match kind {
        EntityKind::Assistant | EntityKind::Workflow => "slug",
        EntityKind::Skill => "name",
        EntityKind::Datasource => "repo_name",
    };
    value
        .pointer(&format!("/metadata/{field}"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

/// Build the `(project, kind, key) → source_path` index from all declarations.
///
/// Fails immediately on the first duplicate natural key
/// (`AppError::Schema`, exit 2).
///
/// Declarations without `metadata.project` are indexed under `""` (empty
/// string) because effective-project resolution is a caller-layer concern
/// (F-002/F-004); validate_graph validates references against the same project
/// string that is present in the declaration.
fn build_graph_index(
    decls: &[ParsedDeclaration],
) -> Result<HashMap<GraphKey, PathBuf>, AppError> {
    let mut index: HashMap<GraphKey, PathBuf> = HashMap::new();

    for decl in decls {
        let project = decl
            .value
            .pointer("/metadata/project")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_owned();

        let key_val = match natural_key_of(&decl.kind, &decl.value) {
            Some(k) => k,
            // Missing natural key: the schema already caught this; skip to
            // avoid confusing downstream diagnostics.
            None => continue,
        };

        let gk = GraphKey {
            project: project.clone(),
            kind: GraphKind::from(&decl.kind),
            key: key_val.clone(),
        };

        if let Some(existing_path) = index.insert(gk.clone(), decl.source_path.clone()) {
            return Err(AppError::Schema(format!(
                "duplicate {} '{}' in project '{}': \
                 first declared in '{}', also declared in '{}' \
                 (FR-004)",
                gk.kind.as_str(),
                key_val,
                project,
                existing_path.display(),
                decl.source_path.display(),
            )));
        }
    }

    Ok(index)
}

// ---------------------------------------------------------------------------
// Reference resolution helpers
// ---------------------------------------------------------------------------

/// Resolve a single cross-entity reference against the index.
///
/// `ref_kind_label` is a human-readable kind name used in the error message.
fn resolve_ref(
    index: &HashMap<GraphKey, PathBuf>,
    project: &str,
    kind: GraphKind,
    key: &str,
    source_path: &Path,
    ref_kind_label: &str,
) -> Result<(), AppError> {
    let gk = GraphKey {
        project: project.to_owned(),
        kind,
        key: key.to_owned(),
    };
    if !index.contains_key(&gk) {
        return Err(AppError::Schema(format!(
            "'{}': unresolved {} reference '{}' in project '{}'; \
             no matching {} declaration found in the repository \
             (FR-004/DR-003)",
            source_path.display(),
            ref_kind_label,
            key,
            project,
            ref_kind_label,
        )));
    }
    Ok(())
}

/// Extract `(project, key_field_value)` from a reference object such as
/// `skillKey`, `assistantKey`, or `datasourceKey`.
///
/// Returns `None` when either field is absent or not a string.
fn ref_pair(obj: &serde_json::Value, key_field: &str) -> Option<(String, String)> {
    let project = obj.get("project")?.as_str()?.to_owned();
    let key = obj.get(key_field)?.as_str()?.to_owned();
    Some((project, key))
}

// ---------------------------------------------------------------------------
// Cross-entity reference validation
// ---------------------------------------------------------------------------

fn validate_all_references(
    decls: &[ParsedDeclaration],
    index: &HashMap<GraphKey, PathBuf>,
) -> Result<(), AppError> {
    for decl in decls {
        match decl.kind {
            EntityKind::Workflow => validate_workflow_refs(decl, index)?,
            EntityKind::Assistant => validate_assistant_refs(decl, index)?,
            // Skill and Datasource have no outgoing cross-entity references in
            // the v1alpha1 schema.
            _ => {}
        }
    }
    Ok(())
}

/// Validate cross-entity references originating from a Workflow declaration.
///
/// Checks each actor in `execution_config.assistants`:
/// - Persisted actors: `assistantRef.{project, slug}` → resolved Assistant.
/// - Inline actors: `skillRefs[].{project, name}` → resolved Skill;
///   `datasourceRefs[].{project, repo_name}` → resolved Datasource.
fn validate_workflow_refs(
    decl: &ParsedDeclaration,
    index: &HashMap<GraphKey, PathBuf>,
) -> Result<(), AppError> {
    let exec = match decl.value.pointer("/spec/execution_config") {
        Some(v) => v,
        None => return Ok(()),
    };

    let actors = exec
        .get("assistants")
        .and_then(|v| v.as_array())
        .map(|a| a.as_slice())
        .unwrap_or(&[]);

    for actor in actors {
        // Persisted actor form: has `assistantRef: {project, slug}`.
        if let Some(assistant_ref) = actor.get("assistantRef") {
            if let Some((project, slug)) = ref_pair(assistant_ref, "slug") {
                resolve_ref(
                    index,
                    &project,
                    GraphKind::Assistant,
                    &slug,
                    &decl.source_path,
                    "Assistant",
                )?;
            }
        }

        // Inline actor form: `skillRefs[]: [{project, name}]`.
        if let Some(skill_refs) = actor.get("skillRefs").and_then(|v| v.as_array()) {
            for sr in skill_refs {
                if let Some((project, name)) = ref_pair(sr, "name") {
                    resolve_ref(
                        index,
                        &project,
                        GraphKind::Skill,
                        &name,
                        &decl.source_path,
                        "Skill",
                    )?;
                }
            }
        }

        // Inline actor form: `datasourceRefs[]: [{project, repo_name}]`.
        if let Some(ds_refs) = actor.get("datasourceRefs").and_then(|v| v.as_array()) {
            for dr in ds_refs {
                if let Some((project, repo_name)) = ref_pair(dr, "repo_name") {
                    resolve_ref(
                        index,
                        &project,
                        GraphKind::Datasource,
                        &repo_name,
                        &decl.source_path,
                        "Datasource",
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Validate cross-entity references originating from an Assistant declaration.
///
/// Checks:
/// - `spec.context[].ref.{project, repo_name}` → resolved Datasource.
/// - `spec.sub_assistants[].{project, slug}` → resolved Assistant.
/// - `spec.skills[].{project, name}` → resolved Skill.
fn validate_assistant_refs(
    decl: &ParsedDeclaration,
    index: &HashMap<GraphKey, PathBuf>,
) -> Result<(), AppError> {
    let spec = match decl.value.pointer("/spec") {
        Some(v) => v,
        None => return Ok(()),
    };

    // context[].ref → datasourceKey {project, repo_name}
    if let Some(context) = spec.get("context").and_then(|v| v.as_array()) {
        for ctx in context {
            if let Some(ref_val) = ctx.get("ref") {
                if let Some((project, repo_name)) = ref_pair(ref_val, "repo_name") {
                    resolve_ref(
                        index,
                        &project,
                        GraphKind::Datasource,
                        &repo_name,
                        &decl.source_path,
                        "Datasource",
                    )?;
                }
            }
        }
    }

    // sub_assistants[] → assistantKey {project, slug}
    if let Some(sub_assts) = spec.get("sub_assistants").and_then(|v| v.as_array()) {
        for sa in sub_assts {
            if let Some((project, slug)) = ref_pair(sa, "slug") {
                resolve_ref(
                    index,
                    &project,
                    GraphKind::Assistant,
                    &slug,
                    &decl.source_path,
                    "Assistant",
                )?;
            }
        }
    }

    // skills[] → skillKey {project, name}
    if let Some(skills) = spec.get("skills").and_then(|v| v.as_array()) {
        for sk in skills {
            if let Some((project, name)) = ref_pair(sk, "name") {
                resolve_ref(
                    index,
                    &project,
                    GraphKind::Skill,
                    &name,
                    &decl.source_path,
                    "Skill",
                )?;
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    /// Minimal Skill declaration with the given project and name.
    fn skill_decl(project: &str, name: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Skill,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Skill",
                "metadata": {"project": project, "name": name},
                "spec": {}
            }),
            source_path: p(&format!("skills/{name}.yaml")),
        }
    }

    /// Minimal Datasource declaration with the given project and repo_name.
    fn datasource_decl(project: &str, repo_name: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Datasource,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Datasource",
                "metadata": {"project": project, "repo_name": repo_name},
                "spec": {}
            }),
            source_path: p(&format!("datasources/{repo_name}.yaml")),
        }
    }

    /// Minimal Assistant declaration with the given project and slug.
    fn assistant_decl(project: &str, slug: &str) -> ParsedDeclaration {
        ParsedDeclaration {
            kind: EntityKind::Assistant,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Assistant",
                "metadata": {"project": project, "slug": slug},
                "spec": {}
            }),
            source_path: p(&format!("assistants/{slug}.yaml")),
        }
    }

    /// Workflow declaration with an inline actor.
    fn workflow_with_inline_actor(
        project: &str,
        slug: &str,
        actor_id: &str,
        skill_refs: &[(&str, &str)],     // (project, name)
        datasource_refs: &[(&str, &str)], // (project, repo_name)
        states_assistant_ids: &[&str],
    ) -> ParsedDeclaration {
        let actors = serde_json::json!([{
            "id": actor_id,
            "system_prompt": "test prompt",
            "skillRefs": skill_refs.iter().map(|(p, n)| serde_json::json!({"project": p, "name": n})).collect::<Vec<_>>(),
            "datasourceRefs": datasource_refs.iter().map(|(p, r)| serde_json::json!({"project": p, "repo_name": r})).collect::<Vec<_>>(),
        }]);

        let states: Vec<serde_json::Value> = states_assistant_ids
            .iter()
            .enumerate()
            .map(|(i, aid)| {
                serde_json::json!({
                    "id": format!("state-{i}"),
                    "assistant_id": aid,
                })
            })
            .collect();

        ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": project, "slug": slug},
                "spec": {
                    "execution_config": {
                        "assistants": actors,
                        "tools": [],
                        "custom_nodes": [],
                        "states": states,
                    }
                }
            }),
            source_path: p(&format!("workflows/{slug}.yaml")),
        }
    }

    /// Workflow declaration with a persisted actor.
    fn workflow_with_persisted_actor(
        project: &str,
        slug: &str,
        actor_id: &str,
        assistant_ref_project: &str,
        assistant_ref_slug: &str,
        states_assistant_ids: &[&str],
    ) -> ParsedDeclaration {
        let actors = serde_json::json!([{
            "id": actor_id,
            "assistantRef": {"project": assistant_ref_project, "slug": assistant_ref_slug},
        }]);

        let states: Vec<serde_json::Value> = states_assistant_ids
            .iter()
            .enumerate()
            .map(|(i, aid)| {
                serde_json::json!({
                    "id": format!("state-{i}"),
                    "assistant_id": aid,
                })
            })
            .collect();

        ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": project, "slug": slug},
                "spec": {
                    "execution_config": {
                        "assistants": actors,
                        "tools": [],
                        "custom_nodes": [],
                        "states": states,
                    }
                }
            }),
            source_path: p(&format!("workflows/{slug}.yaml")),
        }
    }

    /// Workflow declaration with duplicate actor IDs.
    fn workflow_with_duplicate_actor_ids(project: &str, slug: &str, id: &str) -> ParsedDeclaration {
        let actors = serde_json::json!([
            {"id": id, "system_prompt": "first"},
            {"id": id, "system_prompt": "second"},
        ]);
        ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": project, "slug": slug},
                "spec": {
                    "execution_config": {
                        "assistants": actors,
                        "tools": [],
                        "custom_nodes": [],
                        "states": [],
                    }
                }
            }),
            source_path: p(&format!("workflows/{slug}.yaml")),
        }
    }

    /// Assistant declaration that references a Skill and a Datasource.
    fn assistant_with_refs(
        project: &str,
        slug: &str,
        skill_refs: &[(&str, &str)],      // (project, name)
        datasource_refs: &[(&str, &str)], // (project, repo_name)
        sub_assistant_refs: &[(&str, &str)], // (project, slug)
    ) -> ParsedDeclaration {
        let skills: Vec<serde_json::Value> = skill_refs
            .iter()
            .map(|(p, n)| serde_json::json!({"project": p, "name": n}))
            .collect();
        let context: Vec<serde_json::Value> = datasource_refs
            .iter()
            .map(|(p, r)| serde_json::json!({"ref": {"project": p, "repo_name": r}}))
            .collect();
        let sub_assts: Vec<serde_json::Value> = sub_assistant_refs
            .iter()
            .map(|(p, s)| serde_json::json!({"project": p, "slug": s}))
            .collect();
        ParsedDeclaration {
            kind: EntityKind::Assistant,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Assistant",
                "metadata": {"project": project, "slug": slug},
                "spec": {
                    "skills": skills,
                    "context": context,
                    "sub_assistants": sub_assts,
                }
            }),
            source_path: p(&format!("assistants/{slug}.yaml")),
        }
    }

    // -----------------------------------------------------------------------
    // validate_natural — Workflow-local invariants
    // -----------------------------------------------------------------------

    /// A Workflow with no actors and no states passes validate_natural (happy path).
    #[test]
    fn valid_workflow_with_no_actors_passes_natural() {
        let decl = ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": "my-project", "slug": "my-flow"},
                "spec": {
                    "execution_config": {
                        "assistants": [],
                        "tools": [],
                        "custom_nodes": [],
                        "states": []
                    }
                }
            }),
            source_path: p("workflows/my-flow.yaml"),
        };
        let result = validate_natural(&decl);
        assert!(result.is_ok(), "empty workflow must pass natural validation: {result:?}");
    }

    /// A valid slug is accepted by validate_natural (no false positive).
    ///
    /// Slug format (`[a-z0-9][a-z0-9-]*[a-z0-9]`) is enforced at schema
    /// validation time (F-004); validate_natural must not produce false
    /// positives for valid slugs.
    #[test]
    fn valid_slug_accepted_by_validate_natural() {
        let decl = skill_decl("my-project", "my-skill-name");
        let result = validate_natural(&decl);
        assert!(
            result.is_ok(),
            "declaration with valid slug must pass validate_natural: {result:?}"
        );
    }

    /// Non-Workflow kinds pass validate_natural unconditionally (schema handles
    /// their semantic rules).
    #[test]
    fn non_workflow_kinds_pass_validate_natural() {
        let kinds = vec![
            skill_decl("proj", "my-skill-name"),
            datasource_decl("proj", "my-repo"),
            assistant_decl("proj", "my-assistant"),
        ];
        for decl in &kinds {
            let result = validate_natural(decl);
            assert!(
                result.is_ok(),
                "{} must pass validate_natural (no-op): {result:?}",
                decl.kind
            );
        }
    }

    /// A Workflow with one inline actor whose id is referenced by a state
    /// passes validate_natural.
    #[test]
    fn workflow_with_valid_actor_and_state_ref_passes_natural() {
        let decl = workflow_with_inline_actor("p", "my-flow", "actor-1", &[], &[], &["actor-1"]);
        let result = validate_natural(&decl);
        assert!(
            result.is_ok(),
            "valid actor/state relationship must pass: {result:?}"
        );
    }

    /// A Workflow with duplicate actor ids in `execution_config.assistants`
    /// fails with AppError::Schema (exit 2).
    #[test]
    fn workflow_duplicate_actor_id_fails_validate_natural() {
        let decl = workflow_with_duplicate_actor_ids("proj", "wf", "actor-a");
        let err = validate_natural(&decl)
            .expect_err("duplicate actor id must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2, "duplicate actor id must be exit 2");
        let msg = format!("{err}");
        assert!(
            msg.contains("duplicate") || msg.contains("actor"),
            "error must mention duplicate/actor: {msg}"
        );
    }

    /// A Workflow state referencing an actor id not in `execution_config.assistants`
    /// fails with AppError::Schema (exit 2).
    #[test]
    fn workflow_state_refs_missing_actor_id_fails_validate_natural() {
        // State references "nonexistent", but the actor list only has "actor-1".
        let decl =
            workflow_with_inline_actor("p", "my-flow", "actor-1", &[], &[], &["nonexistent"]);
        let err = validate_natural(&decl)
            .expect_err("unresolved state actor_id must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2, "unresolved state reference must be exit 2");
        let msg = format!("{err}");
        assert!(
            msg.contains("nonexistent") || msg.contains("assistant_id"),
            "error must name the unresolved id: {msg}"
        );
    }

    /// A Workflow state referencing an unknown tool_id fails with AppError::Schema.
    #[test]
    fn workflow_state_refs_missing_tool_id_fails_validate_natural() {
        let decl = ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": "p", "slug": "wf"},
                "spec": {
                    "execution_config": {
                        "assistants": [],
                        "tools": [{"id": "tool-1"}],
                        "custom_nodes": [],
                        "states": [{"id": "s1", "tool_id": "missing-tool", "tool_args": {}}],
                    }
                }
            }),
            source_path: p("workflows/wf.yaml"),
        };
        let err = validate_natural(&decl)
            .expect_err("unknown tool_id must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("missing-tool") || msg.contains("tool_id"),
            "error must name the unresolved tool id: {msg}"
        );
    }

    /// A Workflow state referencing an unknown custom_node_id fails.
    #[test]
    fn workflow_state_refs_missing_custom_node_id_fails_validate_natural() {
        let decl = ParsedDeclaration {
            kind: EntityKind::Workflow,
            value: serde_json::json!({
                "apiVersion": "codemie.epam.com/v1alpha1",
                "kind": "Workflow",
                "metadata": {"project": "p", "slug": "wf"},
                "spec": {
                    "execution_config": {
                        "assistants": [],
                        "tools": [],
                        "custom_nodes": [{"id": "node-1"}],
                        "states": [{"id": "s1", "custom_node_id": "ghost-node"}],
                    }
                }
            }),
            source_path: p("workflows/wf.yaml"),
        };
        let err = validate_natural(&decl)
            .expect_err("unknown custom_node_id must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("ghost-node") || msg.contains("custom_node_id"),
            "error must name the unresolved node id: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // validate_graph — duplicate natural key detection
    // -----------------------------------------------------------------------

    /// An empty declaration set passes validate_graph.
    #[test]
    fn empty_declaration_set_passes_validate_graph() {
        let result = validate_graph(&[]);
        assert!(result.is_ok(), "empty set must pass graph validation: {result:?}");
    }

    /// A single declaration with no cross-entity references passes.
    #[test]
    fn single_decl_no_refs_passes_validate_graph() {
        let decl = skill_decl("my-project", "my-skill-name");
        let result = validate_graph(std::slice::from_ref(&decl));
        assert!(result.is_ok(), "single decl with no refs must pass: {result:?}");
    }

    /// Two skills in the same project with the same name are duplicates →
    /// AppError::Schema exit 2.
    #[test]
    fn duplicate_skill_slug_in_same_project_fails_validate_graph() {
        let decls = vec![
            skill_decl("proj-x", "my-skill-name"),
            skill_decl("proj-x", "my-skill-name"),
        ];
        let err = validate_graph(&decls)
            .expect_err("duplicate skill must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2, "duplicate must be exit 2");
        let msg = format!("{err}");
        assert!(
            msg.to_lowercase().contains("duplicate"),
            "error must mention duplicate: {msg}"
        );
    }

    /// Two skills with the same name but different projects are not duplicates.
    #[test]
    fn same_key_different_projects_are_not_duplicates() {
        let decls = vec![
            skill_decl("proj-a", "my-skill-name"),
            skill_decl("proj-b", "my-skill-name"),
        ];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "same name in different projects must not be a duplicate: {result:?}"
        );
    }

    /// A Skill and an Assistant with the same project and same identifier
    /// string are not duplicates (different kinds).
    #[test]
    fn same_key_different_kinds_are_not_duplicates() {
        let decls = vec![
            skill_decl("proj", "my-entity"),
            assistant_decl("proj", "my-entity"),
        ];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "same key for different kinds must not be a duplicate: {result:?}"
        );
    }

    /// Two assistants in the same project with the same slug are duplicates.
    #[test]
    fn duplicate_assistant_slug_fails_validate_graph() {
        let decls = vec![
            assistant_decl("proj", "my-assistant"),
            assistant_decl("proj", "my-assistant"),
        ];
        let err = validate_graph(&decls)
            .expect_err("duplicate assistant must fail");
        assert_eq!(err.exit_code(), 2);
    }

    // -----------------------------------------------------------------------
    // validate_graph — cross-entity reference resolution
    // -----------------------------------------------------------------------

    /// A Workflow that references an existing Skill resolves without error.
    #[test]
    fn workflow_valid_skill_ref_resolves() {
        let skill = skill_decl("proj", "my-skill-name");
        let wf = workflow_with_inline_actor(
            "proj",
            "my-flow",
            "a1",
            &[("proj", "my-skill-name")],
            &[],
            &[],
        );
        let decls = vec![skill, wf];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "resolved skill ref must pass graph validation: {result:?}"
        );
    }

    /// A Workflow that references a non-existent Skill fails with
    /// AppError::Schema exit 2.
    #[test]
    fn workflow_unresolved_skill_ref_fails_validate_graph() {
        let wf = workflow_with_inline_actor(
            "proj",
            "my-flow",
            "a1",
            &[("proj", "missing-skill")],
            &[],
            &[],
        );
        let err = validate_graph(std::slice::from_ref(&wf))
            .expect_err("unresolved skill ref must fail with AppError::Schema");
        assert_eq!(err.exit_code(), 2, "unresolved ref must be exit 2");
        let msg = format!("{err}");
        assert!(
            msg.contains("missing-skill") || msg.contains("Skill"),
            "error must mention the unresolved key: {msg}"
        );
    }

    /// A Workflow that references a non-existent Datasource fails.
    #[test]
    fn workflow_unresolved_datasource_ref_fails_validate_graph() {
        let wf = workflow_with_inline_actor(
            "proj",
            "my-flow",
            "a1",
            &[],
            &[("proj", "ghost-repo")],
            &[],
        );
        let err = validate_graph(std::slice::from_ref(&wf))
            .expect_err("unresolved datasource ref must fail");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("ghost-repo") || msg.contains("Datasource"),
            "error must mention the unresolved key: {msg}"
        );
    }

    /// A Workflow that references an existing Datasource resolves without error.
    #[test]
    fn workflow_valid_datasource_ref_resolves() {
        let ds = datasource_decl("proj", "my-repo");
        let wf = workflow_with_inline_actor(
            "proj",
            "my-flow",
            "a1",
            &[],
            &[("proj", "my-repo")],
            &[],
        );
        let decls = vec![ds, wf];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "resolved datasource ref must pass: {result:?}"
        );
    }

    /// A Workflow persisted actor referencing a non-existent Assistant fails.
    #[test]
    fn workflow_unresolved_assistant_ref_fails_validate_graph() {
        let wf =
            workflow_with_persisted_actor("proj", "my-flow", "a1", "proj", "ghost-assistant", &[]);
        let err = validate_graph(std::slice::from_ref(&wf))
            .expect_err("unresolved assistant ref must fail");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("ghost-assistant") || msg.contains("Assistant"),
            "error must mention the unresolved key: {msg}"
        );
    }

    /// A Workflow persisted actor referencing an existing Assistant resolves.
    #[test]
    fn workflow_valid_persisted_assistant_ref_resolves() {
        let asst = assistant_decl("proj", "my-assistant");
        let wf = workflow_with_persisted_actor(
            "proj",
            "my-flow",
            "a1",
            "proj",
            "my-assistant",
            &["a1"],
        );
        let decls = vec![asst, wf];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "resolved persisted assistant ref must pass: {result:?}"
        );
    }

    /// An Assistant referencing a non-existent Skill via `spec.skills[]` fails.
    #[test]
    fn assistant_unresolved_skill_ref_fails_validate_graph() {
        let asst =
            assistant_with_refs("proj", "my-assistant", &[("proj", "no-skill")], &[], &[]);
        let err = validate_graph(std::slice::from_ref(&asst))
            .expect_err("unresolved assistant skill ref must fail");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("no-skill") || msg.contains("Skill"),
            "error must mention the unresolved key: {msg}"
        );
    }

    /// An Assistant referencing a non-existent Datasource via `spec.context[]` fails.
    #[test]
    fn assistant_unresolved_datasource_context_ref_fails_validate_graph() {
        let asst = assistant_with_refs(
            "proj",
            "my-assistant",
            &[],
            &[("proj", "no-datasource")],
            &[],
        );
        let err = validate_graph(std::slice::from_ref(&asst))
            .expect_err("unresolved context datasource ref must fail");
        assert_eq!(err.exit_code(), 2);
    }

    /// An Assistant referencing a non-existent sub_assistant fails.
    #[test]
    fn assistant_unresolved_sub_assistant_ref_fails_validate_graph() {
        let asst = assistant_with_refs(
            "proj",
            "my-assistant",
            &[],
            &[],
            &[("proj", "missing-sub")],
        );
        let err = validate_graph(std::slice::from_ref(&asst))
            .expect_err("unresolved sub_assistant ref must fail");
        assert_eq!(err.exit_code(), 2);
        let msg = format!("{err}");
        assert!(
            msg.contains("missing-sub") || msg.contains("Assistant"),
            "error must mention the unresolved sub-assistant: {msg}"
        );
    }

    /// An Assistant referencing an existing sub_assistant resolves without error.
    #[test]
    fn assistant_valid_sub_assistant_ref_resolves() {
        let sub = assistant_decl("proj", "sub-assistant");
        let parent = assistant_with_refs(
            "proj",
            "parent-assistant",
            &[],
            &[],
            &[("proj", "sub-assistant")],
        );
        let decls = vec![sub, parent];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "resolved sub_assistant ref must pass: {result:?}"
        );
    }

    /// A cross-project reference resolves when the referenced entity exists in
    /// the declaration set for that other project.
    #[test]
    fn cross_project_reference_resolves_when_entity_exists() {
        let skill = skill_decl("project-b", "shared-skill");
        let asst = assistant_with_refs(
            "project-a",
            "my-assistant",
            &[("project-b", "shared-skill")],
            &[],
            &[],
        );
        let decls = vec![skill, asst];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "cross-project reference to existing entity must resolve: {result:?}"
        );
    }

    /// A cross-project reference fails when the referenced entity is absent.
    #[test]
    fn cross_project_reference_missing_entity_fails_validate_graph() {
        let asst = assistant_with_refs(
            "project-a",
            "my-assistant",
            &[("project-b", "no-such-skill")],
            &[],
            &[],
        );
        let err = validate_graph(std::slice::from_ref(&asst))
            .expect_err("cross-project unresolved ref must fail");
        assert_eq!(err.exit_code(), 2, "cross-project unresolved ref must be exit 2");
        let msg = format!("{err}");
        assert!(
            msg.contains("no-such-skill") || msg.contains("project-b"),
            "error must name the unresolved key or project: {msg}"
        );
    }

    /// Inline sidecar: Skill and Datasource have no outgoing cross-entity refs
    /// so they produce no reference errors (they can only appear as the targets
    /// of refs, not the sources).
    #[test]
    fn skill_and_datasource_have_no_outgoing_refs() {
        let decls = vec![
            skill_decl("proj", "standalone-skill"),
            datasource_decl("proj", "standalone-repo"),
        ];
        let result = validate_graph(&decls);
        assert!(
            result.is_ok(),
            "Skill and Datasource have no outgoing refs: {result:?}"
        );
    }
}
