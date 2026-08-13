//! Shared offline repository loading and graph validation.
//!
//! This is the F-005 implementation boundary used by both `lint` and the
//! local-validation phase of `apply`. It performs deterministic discovery,
//! bounded declaration/sidecar loading, effective-project materialization,
//! natural validation, and whole-repository graph closure without network
//! access.

use std::path::{Path, PathBuf};

use crate::cancellation::CancellationToken;
use crate::discovery::{
    MAX_SIDECAR_FILE_BYTES, discover_yaml_files, load_declaration_file_cancellable,
    load_sidecar_file_cancellable, resolve_sidecar_path,
};
use crate::error::AppError;
use crate::parse::{
    MAX_YAML_FILE_BYTES, ParsedDeclaration, parse_and_validate_cancellable_with_sidecar,
};
use crate::validate::{validate_graph, validate_natural};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepositoryError {
    #[error("cannot canonicalize repository root {path}")]
    RepositoryRoot {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("repository path is unavailable: {path}")]
    PathUnavailable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("generated declaration is not valid UTF-8")]
    GeneratedUtf8(#[source] std::string::FromUtf8Error),
    #[error("overlay path is outside repository: {path}")]
    OutsideRepository {
        path: PathBuf,
        #[source]
        source: std::path::StripPrefixError,
    },
}

impl RepositoryError {
    pub(crate) fn is_configuration(&self) -> bool {
        matches!(
            self,
            Self::RepositoryRoot { .. } | Self::OutsideRepository { .. }
        )
    }
}

pub trait RepositoryView {
    fn yaml_paths(&self) -> Result<Vec<PathBuf>, AppError>;
    fn open_yaml(&self, path: &Path, cancellation: &CancellationToken) -> Result<String, AppError>;
    fn open_sidecar(
        &self,
        owner: &Path,
        relative_path: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, AppError>;
    fn repo_root(&self) -> &Path;
    fn follow_symlinks(&self) -> bool;
}

pub struct DiskRepositoryView {
    repo_root: PathBuf,
    follow_symlinks: bool,
}

impl DiskRepositoryView {
    pub fn new(repo_root: &Path, follow_symlinks: bool) -> Result<Self, AppError> {
        let repo_root =
            std::fs::canonicalize(repo_root).map_err(|source| RepositoryError::RepositoryRoot {
                path: repo_root.to_owned(),
                source,
            })?;
        Ok(Self {
            repo_root,
            follow_symlinks,
        })
    }
}

impl RepositoryView for DiskRepositoryView {
    fn yaml_paths(&self) -> Result<Vec<PathBuf>, AppError> {
        discover_yaml_files(&self.repo_root, self.follow_symlinks).map(|files| {
            files
                .into_iter()
                .map(|file| {
                    if file.byte_len() > MAX_YAML_FILE_BYTES as u64 {
                        return Err(AppError::YamlParse(
                            "declaration exceeds the 1 MiB byte limit".into(),
                        ));
                    }
                    Ok(file.into_path())
                })
                .collect()
        })?
    }

    fn open_yaml(&self, path: &Path, cancellation: &CancellationToken) -> Result<String, AppError> {
        load_declaration_file_cancellable(
            path,
            &self.repo_root,
            self.follow_symlinks,
            MAX_YAML_FILE_BYTES as u64,
            cancellation,
        )
    }

    fn open_sidecar(
        &self,
        owner: &Path,
        relative_path: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, AppError> {
        let path =
            resolve_sidecar_path(relative_path, owner, &self.repo_root, self.follow_symlinks)?;
        load_sidecar_file_cancellable(&path, MAX_SIDECAR_FILE_BYTES, cancellation)
    }

    fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    fn follow_symlinks(&self) -> bool {
        self.follow_symlinks
    }
}

pub struct OverlayRepositoryView {
    disk: DiskRepositoryView,
    yaml_path: PathBuf,
    yaml: Vec<u8>,
    sidecar_path: Option<PathBuf>,
    sidecar: Option<Vec<u8>>,
}

impl OverlayRepositoryView {
    pub fn new(
        repo_root: &Path,
        follow_symlinks: bool,
        yaml_path: &Path,
        yaml: &[u8],
        sidecar_path: Option<&Path>,
        sidecar: Option<&[u8]>,
    ) -> Result<Self, AppError> {
        let disk = DiskRepositoryView::new(repo_root, follow_symlinks)?;
        let yaml_path = canonical_new_path(&disk, yaml_path)?;
        let disk_paths = disk.yaml_paths()?;
        if disk_paths.iter().any(|path| path == &yaml_path)
            || !path_is_new(&yaml_path, disk.repo_root())
        {
            return Err(AppError::EntityNotExportable);
        }
        let (sidecar_path, sidecar) = match (sidecar_path, sidecar) {
            (Some(path), Some(bytes)) => {
                if bytes.len() as u64 > MAX_SIDECAR_FILE_BYTES {
                    return Err(AppError::Schema(
                        "generated Skill sidecar exceeds its byte limit".into(),
                    ));
                }
                let path = canonical_new_path(&disk, path)?;
                if disk_paths.iter().any(|yaml| yaml == &path)
                    || !path_is_new(&path, disk.repo_root())
                {
                    return Err(AppError::EntityNotExportable);
                }
                (Some(path), Some(bytes.to_vec()))
            }
            (None, None) => (None, None),
            _ => {
                return Err(AppError::Internal(
                    "overlay sidecar bytes and path must match".into(),
                ));
            }
        };
        Ok(Self {
            disk,
            yaml_path,
            yaml: yaml.to_vec(),
            sidecar_path,
            sidecar,
        })
    }
}

impl RepositoryView for OverlayRepositoryView {
    fn yaml_paths(&self) -> Result<Vec<PathBuf>, AppError> {
        let mut paths = self.disk.yaml_paths()?;
        paths.push(self.yaml_path.clone());
        paths.sort();
        Ok(paths)
    }

    fn open_yaml(&self, path: &Path, cancellation: &CancellationToken) -> Result<String, AppError> {
        if path == self.yaml_path {
            cancellation.checkpoint()?;
            return String::from_utf8(self.yaml.clone())
                .map_err(RepositoryError::GeneratedUtf8)
                .map_err(AppError::from);
        }
        self.disk.open_yaml(path, cancellation)
    }

    fn open_sidecar(
        &self,
        owner: &Path,
        relative_path: &str,
        cancellation: &CancellationToken,
    ) -> Result<Vec<u8>, AppError> {
        if owner == self.yaml_path {
            let expected = self
                .sidecar_path
                .as_deref()
                .ok_or_else(|| AppError::Schema("generated Skill sidecar is missing".into()))?;
            let requested = owner
                .parent()
                .ok_or_else(|| AppError::Schema("generated declaration path is invalid".into()))?
                .join(relative_path);
            if requested != expected {
                return Err(AppError::Schema(
                    "generated Skill sidecar path is invalid".into(),
                ));
            }
            cancellation.checkpoint()?;
            return self
                .sidecar
                .clone()
                .ok_or_else(|| AppError::Schema("generated Skill sidecar is missing".into()));
        }
        self.disk.open_sidecar(owner, relative_path, cancellation)
    }

    fn repo_root(&self) -> &Path {
        self.disk.repo_root()
    }

    fn follow_symlinks(&self) -> bool {
        self.disk.follow_symlinks()
    }
}

fn canonical_new_path(disk: &DiskRepositoryView, path: &Path) -> Result<PathBuf, AppError> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Configuration("overlay path has no parent".into()))?;
    let filename = path
        .file_name()
        .ok_or_else(|| AppError::Configuration("overlay path has no filename".into()))?;
    let normalized = std::fs::canonicalize(parent)
        .map_err(|source| RepositoryError::PathUnavailable {
            path: parent.to_owned(),
            source,
        })?
        .join(filename);
    let relative = normalized.strip_prefix(&disk.repo_root).map_err(|source| {
        RepositoryError::OutsideRepository {
            path: normalized.clone(),
            source,
        }
    })?;
    if relative.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(AppError::Configuration("overlay path is unsafe".into()));
    }
    Ok(disk.repo_root.join(relative))
}

fn path_is_new(path: &Path, repo_root: &Path) -> bool {
    if std::fs::symlink_metadata(path).is_ok() {
        return false;
    }
    path.parent()
        .and_then(|parent| std::fs::canonicalize(parent).ok())
        .is_some_and(|parent| parent.starts_with(repo_root))
}

/// Inputs for loading and validating one target declaration in its repository.
pub struct TargetLoadRequest<'a> {
    pub file: &'a Path,
    pub repo_root: &'a Path,
    pub default_project: Option<&'a str>,
    pub follow_symlinks: bool,
}

/// Load the target declaration after validating the complete repository graph.
pub fn load_target_declaration(
    request: TargetLoadRequest<'_>,
    cancellation: &CancellationToken,
) -> Result<ParsedDeclaration, AppError> {
    cancellation.checkpoint()?;
    let target_path =
        std::fs::canonicalize(request.file).map_err(|source| RepositoryError::PathUnavailable {
            path: request.file.to_owned(),
            source,
        })?;
    let view = DiskRepositoryView::new(request.repo_root, request.follow_symlinks)?;
    let mut declarations =
        load_repository_declarations(&view, request.default_project, cancellation)?;
    cancellation.checkpoint()?;

    let target_index = declarations
        .iter()
        .position(|declaration| declaration.source_path() == target_path)
        .ok_or_else(|| {
            AppError::Schema(
                "target file is not a discovered YAML declaration in the repository".into(),
            )
        })?;
    Ok(declarations.swap_remove(target_index))
}

fn load_repository_declarations(
    view: &dyn RepositoryView,
    default_project: Option<&str>,
    cancellation: &CancellationToken,
) -> Result<Vec<ParsedDeclaration>, AppError> {
    cancellation.checkpoint()?;
    let files = view.yaml_paths()?;
    let mut declarations = Vec::with_capacity(files.len());
    for file in files {
        cancellation.checkpoint()?;
        let raw = view.open_yaml(&file, cancellation)?;
        let sidecar =
            |owner: &Path, relative: &str| view.open_sidecar(owner, relative, cancellation);
        let mut declaration = parse_and_validate_cancellable_with_sidecar(
            &raw,
            &file,
            view.repo_root(),
            view.follow_symlinks(),
            cancellation,
            &sidecar,
        )?;
        materialize_effective_project(&mut declaration, default_project)?;
        validate_natural(&declaration)?;
        declarations.push(declaration);
    }
    cancellation.checkpoint()?;
    validate_graph(&declarations)?;
    Ok(declarations)
}

pub fn validate_overlay(
    view: &OverlayRepositoryView,
    default_project: Option<&str>,
    cancellation: &CancellationToken,
) -> Result<ParsedDeclaration, AppError> {
    let mut declarations = load_repository_declarations(view, default_project, cancellation)?;
    let target = declarations
        .iter()
        .position(|declaration| declaration.source_path() == view.yaml_path)
        .ok_or_else(|| AppError::Schema("generated declaration was not validated".into()))?;
    Ok(declarations.swap_remove(target))
}

fn materialize_effective_project(
    declaration: &mut ParsedDeclaration,
    default_project: Option<&str>,
) -> Result<(), AppError> {
    if !declaration.has_project() {
        let project = default_project
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Schema(
                    "metadata.project is required when repository project is not configured".into(),
                )
            })?;
        declaration.set_default_project(project)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant_yaml(skill: Option<&str>) -> String {
        let skills = skill.map_or_else(
            || "[]".to_owned(),
            |name| format!("[{{project: demo, name: {name}}}]"),
        );
        format!(
            r#"apiVersion: codemie.epam.com/v1alpha1
kind: Assistant
metadata:
  project: demo
  slug: generated
spec:
  name: Generated Assistant
  system_prompt: Helpful
  llm_model_type: gpt
  type: codemie
  context: []
  toolkits: []
  conversation_starters: []
  shared: false
  mcp_servers: []
  sub_assistants: []
  enabled_builtin_subagents: []
  skills: {skills}
  categories: []
  prompt_variables: []
"#
        )
    }

    fn cancellation() -> CancellationToken {
        CancellationToken::default()
    }

    #[test]
    fn overlay_validates_generated_yaml_bytes() {
        let root = tempfile::tempdir().unwrap();
        let yaml_path = root.path().join("generated.yaml");
        let yaml = assistant_yaml(None);
        let overlay =
            OverlayRepositoryView::new(root.path(), false, &yaml_path, yaml.as_bytes(), None, None)
                .unwrap();

        let declaration = validate_overlay(&overlay, Some("demo"), &cancellation()).unwrap();
        assert_eq!(declaration.kind().to_string(), "Assistant");
        assert!(!yaml_path.exists());
    }

    #[test]
    fn overlay_rejects_missing_dependency() {
        let root = tempfile::tempdir().unwrap();
        let yaml_path = root.path().join("generated.yaml");
        let yaml = assistant_yaml(Some("missing-skill"));
        let overlay =
            OverlayRepositoryView::new(root.path(), false, &yaml_path, yaml.as_bytes(), None, None)
                .unwrap();

        let error = validate_overlay(&overlay, Some("demo"), &cancellation()).unwrap_err();
        assert_eq!(error.exit_code(), 2);
        assert!(!yaml_path.exists());
    }

    #[test]
    fn overlay_expands_skill_sidecar_from_memory() {
        let root = tempfile::tempdir().unwrap();
        let yaml_path = root.path().join("generated.yaml");
        let sidecar_path = root.path().join("generated.md");
        let yaml = r#"apiVersion: codemie.epam.com/v1alpha1
kind: Skill
metadata:
  project: demo
  name: generated-skill
spec:
  description: A sufficiently descriptive skill response for testing.
  visibility: private
  categories: []
  toolkits: []
  mcp_servers: []
  companion_files: []
  enabled_builtin_subagents: []
  contentFrom: generated.md
"#;
        let content = b"This is sufficiently long generated Skill content for the in-memory overlay validation test. It contains enough authored instructions to satisfy the closed declaration schema while remaining deterministic.";
        let overlay = OverlayRepositoryView::new(
            root.path(),
            false,
            &yaml_path,
            yaml.as_bytes(),
            Some(&sidecar_path),
            Some(content),
        )
        .unwrap();

        let declaration = validate_overlay(&overlay, Some("demo"), &cancellation()).unwrap();
        assert_eq!(
            declaration.value()["spec"]["content"],
            String::from_utf8(content.to_vec()).unwrap()
        );
        assert!(!sidecar_path.exists());
    }
}
