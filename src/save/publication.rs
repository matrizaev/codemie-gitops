//! Direct create-new output writing for `save`.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use crate::error::AppError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum SaveError {
    #[error("save output already exists")]
    OutputExists,
    #[error("save output path is invalid")]
    OutputPath,
    #[error("save output write failed")]
    OutputWrite(#[source] std::io::Error),
    #[error("server snapshot contains invalid JSON")]
    SnapshotJson(#[source] serde_json::Error),
    #[error("server snapshot contains invalid YAML")]
    SnapshotYaml(#[source] serde_yaml::Error),
    #[error("server snapshot contains invalid base64")]
    SnapshotBase64(#[source] base64::DecodeError),
    #[error("failed to serialize a validated save declaration")]
    Serialization(#[source] serde_yaml::Error),
    #[error("failed to serialize a typed server snapshot")]
    JsonSerialization(#[source] serde_json::Error),
}

impl SaveError {
    pub(crate) fn is_output_exists(&self) -> bool {
        matches!(self, Self::OutputExists)
    }

    pub(crate) fn is_output_path(&self) -> bool {
        matches!(self, Self::OutputPath)
    }

    pub(crate) fn is_output_write(&self) -> bool {
        matches!(self, Self::OutputWrite(_))
    }

    pub(crate) fn is_compatibility(&self) -> bool {
        matches!(
            self,
            Self::SnapshotJson(_) | Self::SnapshotYaml(_) | Self::SnapshotBase64(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OutputPath(PathBuf);

impl OutputPath {
    pub(super) fn as_path(&self) -> &Path {
        &self.0
    }
}

/// Validate detectable path failures before any server access.
pub(super) fn validate_output_path(path: &Path) -> Result<OutputPath, AppError> {
    if path.as_os_str().is_empty()
        || (matches!(path.components().next(), Some(Component::Prefix(_))) && !path.has_root())
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(SaveError::OutputPath.into());
    }
    match std::fs::symlink_metadata(path) {
        Ok(_) => return Err(SaveError::OutputExists.into()),
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(SaveError::OutputPath.into()),
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    validate_parent_components(parent)?;
    Ok(OutputPath(path.to_owned()))
}

fn validate_parent_components(parent: &Path) -> Result<(), AppError> {
    let mut current = PathBuf::new();
    for component in parent.components() {
        let inspect = match component {
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                false
            }
            Component::RootDir => {
                current.push(component.as_os_str());
                true
            }
            Component::CurDir => {
                if current.as_os_str().is_empty() {
                    current.push(".");
                }
                true
            }
            Component::Normal(segment) => {
                current.push(segment);
                true
            }
            Component::ParentDir => return Err(SaveError::OutputPath.into()),
        };
        if !inspect {
            continue;
        }
        let metadata = std::fs::symlink_metadata(&current).map_err(|_| SaveError::OutputPath)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(SaveError::OutputPath.into());
        }
    }
    Ok(())
}

/// Directly create and fully write the final path without staging or cleanup.
pub(super) fn publish(path: &OutputPath, bytes: &[u8]) -> Result<(), AppError> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path.as_path())
        .map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                SaveError::OutputExists
            } else {
                SaveError::OutputPath
            }
        })?;
    write_created(file, bytes)
}

/// Create empty file-datasource placeholders next to the declaration.
pub(super) fn publish_empty_files(
    declaration_path: &OutputPath,
    relative_paths: &[&str],
) -> Result<(), AppError> {
    let parent = declaration_path
        .as_path()
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut created_directory: Option<PathBuf> = None;
    for relative in relative_paths {
        let relative = Path::new(relative);
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(SaveError::OutputPath.into());
        }
        let target = parent.join(relative);
        let directory = target.parent().ok_or(SaveError::OutputPath)?;
        if created_directory.as_deref() != Some(directory) {
            std::fs::create_dir(directory).map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    SaveError::OutputExists
                } else {
                    SaveError::OutputWrite(source)
                }
            })?;
            created_directory = Some(directory.to_owned());
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(target)
            .map_err(|source| {
                if source.kind() == std::io::ErrorKind::AlreadyExists {
                    SaveError::OutputExists
                } else {
                    SaveError::OutputWrite(source)
                }
            })?;
    }
    Ok(())
}

fn write_created(mut writer: impl Write, bytes: &[u8]) -> Result<(), AppError> {
    writer.write_all(bytes).map_err(SaveError::OutputWrite)?;
    writer.flush().map_err(SaveError::OutputWrite)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_write_creates_only_the_final_file() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("saved.yaml");
        let output = validate_output_path(&target).unwrap();
        publish(&output, b"kind: Skill\n").unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"kind: Skill\n");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn file_datasource_export_creates_empty_relative_placeholders() {
        let directory = tempfile::tempdir().unwrap();
        let declaration = directory.path().join("saved.yaml");
        let output = validate_output_path(&declaration).unwrap();

        publish_empty_files(
            &output,
            &["saved.yaml.files/first.txt", "saved.yaml.files/second.xlsx"],
        )
        .unwrap();

        assert_eq!(
            std::fs::read(directory.path().join("saved.yaml.files/first.txt")).unwrap(),
            b""
        );
        assert_eq!(
            std::fs::read(directory.path().join("saved.yaml.files/second.xlsx")).unwrap(),
            b""
        );
        assert!(!declaration.exists());
    }

    #[test]
    fn direct_write_never_replaces_an_existing_target() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("saved.yaml");
        std::fs::write(&target, b"existing\n").unwrap();

        let error = validate_output_path(&target).unwrap_err();
        assert!(matches!(
            error,
            AppError::SaveLayer(SaveError::OutputExists)
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"existing\n");
    }

    #[test]
    fn direct_write_rejects_a_target_created_after_preflight() {
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("saved.yaml");
        let output = validate_output_path(&target).unwrap();
        std::fs::write(&target, b"racing writer\n").unwrap();

        let error = publish(&output, b"replacement\n").unwrap_err();
        assert!(matches!(
            error,
            AppError::SaveLayer(SaveError::OutputExists)
        ));
        assert_eq!(std::fs::read(&target).unwrap(), b"racing writer\n");
    }

    struct FailingWriter {
        fail_flush: bool,
    }

    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if self.fail_flush {
                Ok(bytes.len())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::WriteZero,
                    "injected write failure",
                ))
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            if self.fail_flush {
                Err(std::io::Error::other("injected flush failure"))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn post_creation_write_failure_is_classified_as_output_write() {
        let error = write_created(FailingWriter { fail_flush: false }, b"yaml").unwrap_err();
        assert!(matches!(
            error,
            AppError::SaveLayer(SaveError::OutputWrite(_))
        ));
    }

    #[test]
    fn post_creation_flush_failure_is_classified_as_output_write() {
        let error = write_created(FailingWriter { fail_flush: true }, b"yaml").unwrap_err();
        assert!(matches!(
            error,
            AppError::SaveLayer(SaveError::OutputWrite(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn preflight_rejects_a_symlinked_parent_component() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let real = directory.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let linked = directory.path().join("linked");
        symlink(&real, &linked).unwrap();

        let error = validate_output_path(&linked.join("saved.yaml")).unwrap_err();
        assert!(matches!(error, AppError::SaveLayer(SaveError::OutputPath)));
    }
}
