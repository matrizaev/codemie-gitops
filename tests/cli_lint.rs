use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn assistant_yaml() -> &'static str {
    concat!(
        "apiVersion: codemie.epam.com/v1alpha1\n",
        "kind: Assistant\n",
        "metadata:\n",
        "  project: project-a\n",
        "  slug: assistant-a\n",
        "spec:\n",
        "  name: Assistant A\n",
        "  system_prompt: Helpful\n",
        "  llm_model_type: gpt\n",
        "  type: codemie\n",
        "  context: []\n",
        "  toolkits: []\n",
        "  conversation_starters: []\n",
        "  shared: false\n",
        "  mcp_servers: []\n",
        "  sub_assistants: []\n",
        "  enabled_builtin_subagents: []\n",
        "  prompt_variables: []\n",
        "  skills:\n",
        "    - project: project-a\n",
        "      name: shared-skill\n",
        "  categories: []\n",
    )
}

fn skill_yaml() -> &'static str {
    r#"apiVersion: codemie.epam.com/v1alpha1
kind: Skill
metadata:
  project: project-a
  name: shared-skill
spec:
  description: "A shared skill with a sufficiently descriptive explanation."
  visibility: private
  categories: []
  toolkits: []
  mcp_servers: []
  companion_files: []
  enabled_builtin_subagents: []
  content: "This is valid skill content that is deliberately longer than one hundred characters so the closed declaration schema accepts it during offline lint."
"#
}

fn skill_content_from_yaml() -> &'static str {
    r#"apiVersion: codemie.epam.com/v1alpha1
kind: Skill
metadata:
  project: project-a
  name: sidecar-skill
spec:
  description: "A sidecar skill with a sufficiently descriptive explanation."
  visibility: private
  categories: []
  toolkits: []
  mcp_servers: []
  companion_files: []
  enabled_builtin_subagents: []
  contentFrom: content.md
"#
}

fn assistant_with_custom_metadata_key(key: &str) -> String {
    assistant_with_custom_metadata_keys(&[key])
}

fn assistant_with_custom_metadata_keys(keys: &[&str]) -> String {
    let entries = keys
        .iter()
        .map(|key| {
            let quoted_key = serde_json::to_string(key).expect("metadata key must serialize");
            format!("    {quoted_key}: \"A9!vK2@qP7#xR4$mN8%z\"")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assistant_yaml().replace(
        "  categories: []\n",
        &format!("  categories: []\n  custom_metadata:\n{entries}\n"),
    )
}

fn assistant_wrong_kind_target_yaml() -> String {
    assistant_yaml()
        .replace("slug: assistant-a", "slug: shared-skill")
        .replace(
            "  skills:\n    - project: project-a\n      name: shared-skill\n",
            "  skills: []\n",
        )
}

fn autonomous_workflow_with_secret_yaml() -> &'static str {
    r#"apiVersion: codemie.epam.com/v1alpha1
kind: Workflow
metadata:
  project: project-a
  slug: workflow-a
spec:
  name: Workflow A
  description: A valid workflow for deterministic warning order
  mode: Autonomous
  shared: false
  supervisor_prompt: Coordinate the workflow
  meta_config:
    api_key: "A9!vK2@qP7#xR4$mN8%z"
  execution_config:
    messages_limit_before_summarization: 10
    tokens_limit_before_summarization: 1000
    type: default
    enable_summarization_node: false
    recursion_limit: 10
    max_concurrency: 1
    verbose: false
    max_iteration_key_output_limit: 100
    assistants: []
    tools: []
    custom_nodes: []
    states: []
    retry_policy:
      initial_interval: 1000
      backoff_factor: 2
      max_interval: 60000
      max_attempts: 3
"#
}

fn run_lint(root: &Path, file: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codemie-gitops"))
        .args([
            "lint",
            "--file",
            file.to_str().expect("test path must be UTF-8"),
            "--repo-root",
            root.to_str().expect("test path must be UTF-8"),
            "--output",
            "json",
        ])
        // An unreachable endpoint makes accidental online behavior fail the test.
        .env("CODEMIE_URL", "http://127.0.0.1:9")
        .env("CODEMIE_TOKEN", "must-not-be-used-by-lint")
        .env_remove("RUST_LOG")
        .output()
        .expect("lint process must start")
}

#[test]
fn lint_resolves_repository_reference_offline_and_emits_closed_success() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let assistant = root.path().join("assistant.yaml");
    fs::write(&assistant, assistant_yaml()).expect("assistant fixture must write");
    fs::write(root.path().join("skill.yaml"), skill_yaml()).expect("skill fixture must write");

    let output = run_lint(root.path(), &assistant);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout must be UTF-8"),
        "{\"action\":\"valid\",\"kind\":\"Assistant\",\"project\":\"project-a\",\"slug\":\"assistant-a\"}\n"
    );
    assert!(
        output.stderr.is_empty(),
        "successful lint must have no diagnostic"
    );
}

#[test]
fn lint_emits_warnings_for_target_only() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let skill = root.path().join("skill.yaml");
    fs::write(&skill, skill_yaml()).expect("skill fixture must write");
    fs::write(
        root.path().join("assistant.yaml"),
        assistant_with_custom_metadata_key("api_key"),
    )
    .expect("assistant fixture must write");

    let output = run_lint(root.path(), &skill);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "non-target declarations must not emit warnings"
    );
}

#[test]
fn lint_orders_target_warnings_by_code_then_canonical_path() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let workflow = root.path().join("workflow.yaml");
    fs::write(&workflow, autonomous_workflow_with_secret_yaml())
        .expect("workflow fixture must write");

    let output = run_lint(root.path(), &workflow);

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let warnings: Vec<serde_json::Value> = String::from_utf8(output.stderr)
        .expect("stderr must be UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("warning line must be JSON"))
        .collect();
    assert_eq!(warnings.len(), 2);
    assert_eq!(warnings[0]["source"]["fieldPath"], "spec.mode");
    assert_eq!(warnings[0]["warningCode"], "W_DEPRECATED_VALUE");
    assert_eq!(
        warnings[1]["source"]["fieldPath"],
        "spec.meta_config.api_key"
    );
    assert_eq!(warnings[1]["warningCode"], "W_SUSPECTED_PLAINTEXT_SECRET");
    for warning in &warnings {
        assert_warning_matches_contract(warning);
    }
}

#[test]
fn lint_missing_repository_reference_has_empty_stdout_and_closed_diagnostic() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let assistant = root.path().join("assistant.yaml");
    fs::write(&assistant, assistant_with_custom_metadata_key("api_key"))
        .expect("assistant fixture must write");

    let output = run_lint(root.path(), &assistant);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "failure must leave stdout empty");
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr)
        .expect("stderr must contain exactly one JSON diagnostic");
    assert_eq!(
        diagnostic,
        serde_json::json!({
            "errorCode": "E_SCHEMA",
            "category": "local-input",
            "exitCode": 2
        })
    );
}

#[test]
fn lint_invalid_non_target_closure_declaration_suppresses_target_warning() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let assistant = root.path().join("assistant.yaml");
    fs::write(&assistant, assistant_with_custom_metadata_key("api_key"))
        .expect("warning-bearing target fixture must write");
    fs::write(root.path().join("skill.yaml"), skill_yaml())
        .expect("valid reference fixture must write");
    fs::write(
        root.path().join("invalid-non-target.yaml"),
        "apiVersion: codemie.epam.com/v1alpha1\nkind: Skill\ninvalid: true\n",
    )
    .expect("invalid non-target fixture must write");

    let output = run_lint(root.path(), &assistant);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "failure must leave stdout empty");
    let stderr = String::from_utf8(output.stderr).expect("stderr must be UTF-8");
    assert_eq!(
        stderr.lines().count(),
        1,
        "failure emits one diagnostic only"
    );
    assert!(
        !stderr.contains("W_SUSPECTED_PLAINTEXT_SECRET"),
        "closure failure must suppress target warnings"
    );
    assert!(
        !stderr.contains("A9!vK2@qP7#xR4$mN8%z"),
        "target canary value must never enter output"
    );
    let diagnostic: serde_json::Value =
        serde_json::from_str(&stderr).expect("stderr must contain one JSON diagnostic");
    assert_eq!(
        diagnostic,
        serde_json::json!({
            "errorCode": "E_SCHEMA",
            "category": "local-input",
            "exitCode": 2
        })
    );
}

#[test]
fn lint_rejects_sidecar_content_outside_closed_schema_bounds_without_echoing_it() {
    for (length, canary) in [(99, "BelowMinimumA9!"), (30_001, "AboveMaximumZ7@")] {
        let root = tempfile::tempdir().expect("temporary repository must construct");
        fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
        let skill = root.path().join("skill.yaml");
        fs::write(&skill, skill_content_from_yaml()).expect("skill fixture must write");
        let content = format!("{canary}{}", "x".repeat(length - canary.len()));
        fs::write(root.path().join("content.md"), &content).expect("sidecar must write");

        let output = run_lint(root.path(), &skill);

        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty(), "failure must leave stdout empty");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains(canary),
            "sidecar content must not enter diagnostics"
        );
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr)
            .expect("stderr must contain exactly one JSON diagnostic");
        assert_eq!(diagnostic["errorCode"], "E_SCHEMA");
        assert_eq!(diagnostic["category"], "local-input");
        assert_eq!(diagnostic["exitCode"], 2);
    }
}

#[test]
fn lint_warns_for_reachable_custom_metadata_credential_with_schema_valid_output() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let assistant = root.path().join("assistant.yaml");
    fs::write(
        &assistant,
        assistant_with_custom_metadata_keys(&["z_token", "api_key"]),
    )
    .expect("assistant fixture must write");
    fs::write(root.path().join("skill.yaml"), skill_yaml()).expect("skill fixture must write");

    let output = run_lint(root.path(), &assistant);

    assert!(output.status.success());
    let warnings: Vec<serde_json::Value> = String::from_utf8(output.stderr)
        .expect("stderr must be UTF-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("warning line must be JSON"))
        .collect();
    assert_eq!(warnings.len(), 2);
    assert_eq!(
        warnings[0],
        serde_json::json!({
            "warningCode": "W_SUSPECTED_PLAINTEXT_SECRET",
            "category": "secret-like-field",
            "source": {
                "file": "assistant.yaml",
                "fieldPath": "spec.custom_metadata.api_key"
            }
        })
    );
    assert_eq!(
        warnings[1]["source"]["fieldPath"],
        "spec.custom_metadata.z_token"
    );
    for warning in &warnings {
        assert_warning_matches_contract(warning);
    }
}

#[test]
fn lint_collapses_long_and_control_metadata_keys_to_safe_warning_path() {
    let unsafe_keys = [
        // Stays within YAML's simple-key budget while making the complete
        // canonical field path exceed warning.schema's 1,024-character limit.
        format!("api_key{}", "x".repeat(1_000)),
        "api\u{0001}key".to_owned(),
        "api\u{202e}key".to_owned(),
    ];
    for key in unsafe_keys {
        let root = tempfile::tempdir().expect("temporary repository must construct");
        fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
        let assistant = root.path().join("assistant.yaml");
        fs::write(&assistant, assistant_with_custom_metadata_key(&key))
            .expect("assistant fixture must write");
        fs::write(root.path().join("skill.yaml"), skill_yaml()).expect("skill fixture must write");

        let output = run_lint(root.path(), &assistant);

        assert!(
            output.status.success(),
            "key {:?}, stderr: {}",
            key.escape_debug().to_string(),
            String::from_utf8_lossy(&output.stderr)
        );
        let warning: serde_json::Value = serde_json::from_slice(&output.stderr)
            .expect("stderr must contain exactly one JSON warning");
        assert_eq!(warning["source"]["fieldPath"], "spec.custom_metadata");
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains(&key),
            "unsafe arbitrary metadata key must not enter warning output"
        );
        assert_warning_matches_contract(&warning);
    }
}

#[test]
fn lint_existing_wrong_kind_entity_does_not_satisfy_natural_reference() {
    let root = tempfile::tempdir().expect("temporary repository must construct");
    fs::create_dir(root.path().join(".git")).expect(".git directory must construct");
    let assistant = root.path().join("assistant.yaml");
    fs::write(&assistant, assistant_yaml()).expect("assistant fixture must write");
    fs::write(
        root.path().join("wrong-kind.yaml"),
        assistant_wrong_kind_target_yaml(),
    )
    .expect("wrong-kind fixture must write");

    let output = run_lint(root.path(), &assistant);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty(), "failure must leave stdout empty");
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr)
        .expect("stderr must contain exactly one JSON diagnostic");
    assert_eq!(diagnostic["errorCode"], "E_SCHEMA");
    assert_eq!(diagnostic["category"], "local-input");
    assert_eq!(diagnostic["exitCode"], 2);
}

fn assert_warning_matches_contract(warning: &serde_json::Value) {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../specs/codemie-cicd-tool/contracts/warning.schema.json"
    ))
    .expect("warning schema must parse");
    let validator = jsonschema::validator_for(&schema).expect("warning schema must compile");
    let errors: Vec<_> = validator
        .iter_errors(warning)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "warning must match contract: {errors:?}");
}
