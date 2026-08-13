use std::fs;
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
        "      name: server-skill\n",
        "  categories: []\n",
    )
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

fn run_lint(file: &std::path::Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_codemie-gitops"))
        .args(["lint", "--file"])
        .arg(file)
        .args(["--output", "json"])
        .env("CODEMIE_URL", "http://127.0.0.1:9")
        .env("CODEMIE_TOKEN", "must-not-be-used-by-lint")
        .env_remove("RUST_LOG")
        .output()
        .expect("lint process must start")
}

#[test]
fn lint_shape_checks_references_without_neighbor_declarations() {
    let directory = tempfile::tempdir().expect("temporary directory must construct");
    let selected = directory.path().join("assistant.yaml");
    fs::write(&selected, assistant_yaml()).expect("fixture must write");

    let output = run_lint(&selected);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let outcome: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(outcome["action"], "valid");
}

#[test]
fn lint_ignores_invalid_neighbor_yaml() {
    let directory = tempfile::tempdir().expect("temporary directory must construct");
    let selected = directory.path().join("assistant.yaml");
    fs::write(&selected, assistant_yaml()).expect("fixture must write");
    fs::write(directory.path().join("invalid.yaml"), "not: [valid").unwrap();

    assert_eq!(run_lint(&selected).status.code(), Some(0));
}

#[test]
fn lint_reads_explicit_skill_markdown_relative_to_selected_file() {
    let directory = tempfile::tempdir().expect("temporary directory must construct");
    let selected = directory.path().join("skill.yaml");
    fs::write(&selected, skill_content_from_yaml()).unwrap();
    fs::write(directory.path().join("content.md"), "x".repeat(100)).unwrap();

    assert_eq!(run_lint(&selected).status.code(), Some(0));
}

#[test]
fn lint_rejects_symlinked_skill_content() {
    let directory = tempfile::tempdir().expect("temporary directory must construct");
    let selected = directory.path().join("skill.yaml");
    fs::write(&selected, skill_content_from_yaml()).unwrap();
    fs::write(directory.path().join("real.md"), "x".repeat(100)).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("real.md", directory.path().join("content.md")).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file("real.md", directory.path().join("content.md")).unwrap();

    let output = run_lint(&selected);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn lint_rejects_selected_yaml_larger_than_one_mibibyte() {
    let directory = tempfile::tempdir().expect("temporary directory must construct");
    let selected = directory.path().join("oversized.yaml");
    fs::write(&selected, vec![b'x'; 1024 * 1024 + 1]).unwrap();

    let output = run_lint(&selected);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn removed_repository_flags_are_unknown() {
    for flag in ["--repo-root", "--follow-symlinks"] {
        let output = Command::new(env!("CARGO_BIN_EXE_codemie-gitops"))
            .args(["lint", "--file", "declaration.yaml", flag])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{flag} must be rejected");
    }
}
