//! Shared offline repository loading and graph validation.
//!
//! This is the F-005 implementation boundary used by both `lint` and the
//! local-validation phase of `apply`. It performs deterministic discovery,
//! bounded declaration/sidecar loading, effective-project materialization,
//! natural validation, and whole-repository graph closure without network
//! access.

use std::path::Path;

use crate::cancellation::CancellationToken;
use crate::discovery::{discover_yaml_files, load_declaration_file_cancellable};
use crate::error::AppError;
use crate::parse::{MAX_YAML_FILE_BYTES, ParsedDeclaration, parse_and_validate_cancellable};
use crate::validate::{validate_graph, validate_natural};

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
    let target_path = std::fs::canonicalize(request.file)
        .map_err(|_| AppError::Schema("target declaration file is unavailable".into()))?;
    let mut declarations = load_repository_declarations(&request, cancellation)?;
    validate_graph(&declarations)?;
    cancellation.checkpoint()?;

    let target_index = declarations
        .iter()
        .position(|declaration| declaration.source_path == target_path)
        .ok_or_else(|| {
            AppError::Schema(
                "target file is not a discovered YAML declaration in the repository".into(),
            )
        })?;
    Ok(declarations.swap_remove(target_index))
}

fn load_repository_declarations(
    request: &TargetLoadRequest<'_>,
    cancellation: &CancellationToken,
) -> Result<Vec<ParsedDeclaration>, AppError> {
    cancellation.checkpoint()?;
    let files = discover_yaml_files(request.repo_root, request.follow_symlinks)?;
    let mut declarations = Vec::with_capacity(files.len());
    for file in files {
        cancellation.checkpoint()?;
        if file.byte_len > MAX_YAML_FILE_BYTES as u64 {
            return Err(AppError::YamlParse(
                "declaration exceeds the 1 MiB byte limit".into(),
            ));
        }
        let raw = load_declaration_file_cancellable(
            &file.path,
            request.repo_root,
            request.follow_symlinks,
            MAX_YAML_FILE_BYTES as u64,
            cancellation,
        )?;
        let mut declaration = parse_and_validate_cancellable(
            &raw,
            &file.path,
            request.repo_root,
            request.follow_symlinks,
            cancellation,
        )?;
        materialize_effective_project(&mut declaration, request.default_project)?;
        validate_natural(&declaration)?;
        declarations.push(declaration);
    }
    cancellation.checkpoint()?;
    Ok(declarations)
}

fn materialize_effective_project(
    declaration: &mut ParsedDeclaration,
    default_project: Option<&str>,
) -> Result<(), AppError> {
    let metadata = declaration
        .value
        .get_mut("metadata")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or_else(|| AppError::Schema("declaration metadata is required".into()))?;
    if metadata.get("project").is_none() {
        let project = default_project
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::Schema(
                    "metadata.project is required when repository project is not configured".into(),
                )
            })?;
        metadata.insert(
            "project".to_owned(),
            serde_json::Value::String(project.to_owned()),
        );
    }
    Ok(())
}
