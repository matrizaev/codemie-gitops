//! Strict loading for one selected declaration and its explicit auxiliary inputs.

use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use crate::error::AppError;
use crate::parse::{MAX_YAML_FILE_BYTES, ParsedDeclaration, parse_and_validate_with_sidecar};
use crate::validate::validate_natural;

const MAX_SKILL_CONTENT_BYTES: u64 = 128 * 1024;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_AGGREGATE_FILE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_FILE_PARTS: usize = 10;

/// Exact bytes and safe multipart name for one authored File Datasource input.
#[derive(Debug)]
pub(crate) struct FilePart {
    pub(crate) filename: String,
    pub(crate) bytes: Vec<u8>,
}

/// One validated declaration plus any explicitly authored File Datasource bytes.
#[derive(Debug)]
pub(crate) struct LoadedDeclaration {
    pub(crate) declaration: ParsedDeclaration,
    pub(crate) file_parts: Vec<FilePart>,
}

/// Read and validate exactly the selected declaration and its approved auxiliaries.
pub(crate) fn load(file: &Path) -> Result<LoadedDeclaration, AppError> {
    let metadata =
        std::fs::symlink_metadata(file).map_err(|source| InputError::OpenSelected { source })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(InputError::SelectedNotRegular.into());
    }
    let raw = read_bounded(file, MAX_YAML_FILE_BYTES as u64, InputKind::Selected)?;
    let raw = String::from_utf8(raw).map_err(|source| InputError::SelectedUtf8 { source })?;
    let parent = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent =
        std::fs::canonicalize(parent).map_err(|source| InputError::DeclarationParent { source })?;
    let sidecar = |relative: &str| {
        let path = resolve_explicit_path(&parent, relative, AuxiliaryKind::Skill)?;
        read_bounded(&path, MAX_SKILL_CONTENT_BYTES, InputKind::Skill)
    };
    let declaration = parse_and_validate_with_sidecar(&raw, file, &sidecar)?;
    validate_natural(&declaration)?;
    let file_parts = load_file_parts(&declaration, &parent)?;
    Ok(LoadedDeclaration {
        declaration,
        file_parts,
    })
}

/// Validate generated inline YAML without granting filesystem access.
pub(crate) fn validate_generated(raw: &str) -> Result<ParsedDeclaration, AppError> {
    let reject_sidecar = |_relative: &str| {
        Err(AppError::Schema(
            "generated declarations must contain inline Skill content".into(),
        ))
    };
    let declaration =
        parse_and_validate_with_sidecar(raw, Path::new("generated.yaml"), &reject_sidecar)?;
    validate_natural(&declaration)?;
    Ok(declaration)
}

fn load_file_parts(
    declaration: &ParsedDeclaration,
    parent: &Path,
) -> Result<Vec<FilePart>, AppError> {
    let Some(paths) = declaration.file_datasource_paths() else {
        return Ok(Vec::new());
    };
    if paths.is_empty() || paths.len() > MAX_FILE_PARTS {
        return Err(AppError::Schema(
            "file datasource must name between one and ten files".into(),
        ));
    }

    let mut seen = HashSet::with_capacity(paths.len());
    let mut aggregate = 0u64;
    let mut parts = Vec::with_capacity(paths.len());
    for authored in paths {
        let path = resolve_explicit_path(parent, authored, AuxiliaryKind::File)?;
        if !seen.insert(path.clone()) {
            return Err(AppError::Schema(
                "file datasource contains duplicate file targets".into(),
            ));
        }
        let filename = safe_basename(&path)?;
        let bytes = read_bounded(&path, MAX_FILE_BYTES, InputKind::File)?;
        aggregate = aggregate
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| AppError::Schema("file datasource aggregate is too large".into()))?;
        if aggregate > MAX_AGGREGATE_FILE_BYTES {
            return Err(AppError::Schema(
                "file datasource aggregate exceeds 128 MiB".into(),
            ));
        }
        parts.push(FilePart { filename, bytes });
    }
    Ok(parts)
}

#[derive(Clone, Copy)]
enum AuxiliaryKind {
    Skill,
    File,
}

fn resolve_explicit_path(
    parent: &Path,
    authored: &str,
    kind: AuxiliaryKind,
) -> Result<PathBuf, AppError> {
    if authored.is_empty() {
        return Err(InputError::UnsafeAuxiliary.into());
    }
    let relative = Path::new(authored);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(InputError::UnsafeAuxiliary.into());
    }
    if matches!(kind, AuxiliaryKind::Skill)
        && relative
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("md")
    {
        return Err(AppError::Schema(
            "Skill contentFrom must name a relative .md file".into(),
        ));
    }

    let mut candidate = parent.to_owned();
    for component in relative.components() {
        let component = match component {
            Component::CurDir => continue,
            Component::Normal(component) => component,
            _ => return Err(InputError::UnsafeAuxiliary.into()),
        };
        candidate.push(component);
        let metadata = std::fs::symlink_metadata(&candidate)
            .map_err(|source| InputError::OpenAuxiliary { source })?;
        if metadata.file_type().is_symlink() {
            return Err(InputError::AuxiliarySymlink.into());
        }
    }
    let metadata = std::fs::symlink_metadata(&candidate)
        .map_err(|source| InputError::OpenAuxiliary { source })?;
    if !metadata.is_file() {
        return Err(InputError::AuxiliaryNotRegular.into());
    }
    let canonical =
        std::fs::canonicalize(&candidate).map_err(|source| InputError::OpenAuxiliary { source })?;
    if !canonical.starts_with(parent) {
        return Err(InputError::UnsafeAuxiliary.into());
    }
    Ok(canonical)
}

fn safe_basename(path: &Path) -> Result<String, AppError> {
    let basename = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or(InputError::UnsafeBasename)?;
    if basename.chars().any(|character| {
        let codepoint = character as u32;
        codepoint <= 0x1f || (0x7f..=0x9f).contains(&codepoint) || matches!(character, '/' | '\\')
    }) {
        return Err(InputError::UnsafeBasename.into());
    }
    Ok(basename.to_owned())
}

#[derive(Clone, Copy)]
pub(crate) enum InputKind {
    Selected,
    Skill,
    File,
}

fn read_bounded(path: &Path, limit: u64, kind: InputKind) -> Result<Vec<u8>, AppError> {
    let file = File::open(path).map_err(|source| InputError::Read { kind, source })?;
    let metadata = file
        .metadata()
        .map_err(|source| InputError::Read { kind, source })?;
    if !metadata.is_file() {
        return Err(InputError::NotRegular { kind }.into());
    }
    if metadata.len() > limit {
        return Err(InputError::TooLarge { kind }.into());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| InputError::Read { kind, source })?;
    if bytes.len() as u64 > limit {
        return Err(InputError::TooLarge { kind }.into());
    }
    Ok(bytes)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InputError {
    #[error("selected declaration cannot be opened")]
    OpenSelected {
        #[source]
        source: std::io::Error,
    },
    #[error("selected declaration must be a regular file")]
    SelectedNotRegular,
    #[error("selected declaration is not UTF-8")]
    SelectedUtf8 {
        #[source]
        source: std::string::FromUtf8Error,
    },
    #[error("selected declaration parent is unavailable")]
    DeclarationParent {
        #[source]
        source: std::io::Error,
    },
    #[error("explicit auxiliary path is unsafe")]
    UnsafeAuxiliary,
    #[error("explicit auxiliary path cannot be opened")]
    OpenAuxiliary {
        #[source]
        source: std::io::Error,
    },
    #[error("explicit auxiliary path contains a symbolic link")]
    AuxiliarySymlink,
    #[error("explicit auxiliary input must be a regular file")]
    AuxiliaryNotRegular,
    #[error("file datasource filename is unsafe")]
    UnsafeBasename,
    #[error("bounded input read failed")]
    Read {
        kind: InputKind,
        #[source]
        source: std::io::Error,
    },
    #[error("bounded input is not a regular file")]
    NotRegular { kind: InputKind },
    #[error("bounded input exceeds its byte limit")]
    TooLarge { kind: InputKind },
}

impl std::fmt::Debug for InputKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Selected => "selected declaration",
            Self::Skill => "Skill content",
            Self::File => "File Datasource input",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_datasource(paths: &[&str]) -> String {
        let files = paths
            .iter()
            .map(|path| format!("    - {path}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata:
  project: project-a
  repo_name: uploaded-documents
spec:
  index_type: file
  description: Explicit file inputs
  files:
{files}
  include_email_attachments: false
"#
        )
    }

    #[test]
    fn file_datasource_reads_exact_authored_files_in_order() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("docs")).unwrap();
        std::fs::write(directory.path().join("docs/a.txt"), b"a").unwrap();
        std::fs::write(directory.path().join("docs/b.txt"), b"bb").unwrap();
        let declaration = directory.path().join("datasource.yaml");
        std::fs::write(&declaration, file_datasource(&["docs/a.txt", "docs/b.txt"])).unwrap();

        let loaded = load(&declaration).unwrap();
        assert_eq!(loaded.file_parts.len(), 2);
        assert_eq!(loaded.file_parts[0].filename, "a.txt");
        assert_eq!(loaded.file_parts[0].bytes, b"a");
        assert_eq!(loaded.file_parts[1].filename, "b.txt");
        assert_eq!(loaded.file_parts[1].bytes, b"bb");
    }

    #[test]
    fn file_datasource_rejects_duplicate_canonical_targets() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("a.txt"), b"a").unwrap();
        let declaration = directory.path().join("datasource.yaml");
        std::fs::write(&declaration, file_datasource(&["a.txt", "./a.txt"])).unwrap();

        assert!(load(&declaration).is_err());
    }

    #[test]
    fn file_datasource_rejects_lexical_escape() {
        let directory = tempfile::tempdir().unwrap();
        let declaration = directory.path().join("datasource.yaml");
        std::fs::write(&declaration, file_datasource(&["../outside.txt"])).unwrap();

        assert!(load(&declaration).is_err());
    }
}
