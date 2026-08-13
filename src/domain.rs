use std::str::FromStr;

use crate::error::AppError;
use crate::parse::{EntityKind, ParsedDeclaration, ParsedNaturalIdentity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputFile(PathBuf);

use std::path::{Path, PathBuf};

impl InputFile {
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

macro_rules! validated_path {
    ($name:ident, $label:literal) => {
        impl TryFrom<PathBuf> for $name {
            type Error = InvalidPath;

            fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
                if value.as_os_str().is_empty() {
                    Err(InvalidPath($label))
                } else {
                    Ok(Self(value))
                }
            }
        }
    };
}

validated_path!(InputFile, "input file");

#[derive(Debug, thiserror::Error)]
#[error("{0} path is empty")]
pub(crate) struct InvalidPath(&'static str);

/// Canonical server workflow identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct WorkflowId(uuid::Uuid);

impl FromStr for WorkflowId {
    type Err = InvalidWorkflowId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        uuid::Uuid::parse_str(value)
            .map(Self)
            .map_err(|source| InvalidWorkflowId { source })
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("workflow ID is not a canonical UUID")]
pub(crate) struct InvalidWorkflowId {
    #[source]
    source: uuid::Error,
}

/// Non-empty server-owned entity identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ServerId(String);

impl ServerId {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ServerId {
    type Error = InvalidServerId;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() {
            Err(InvalidServerId)
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("server ID is empty")]
pub(crate) struct InvalidServerId;

macro_rules! non_empty_identity {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct $name(String);

        impl TryFrom<String> for $name {
            type Error = InvalidIdentityField;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.is_empty() {
                    Err(InvalidIdentityField { field: $label })
                } else {
                    Ok(Self(value))
                }
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }
    };
}

non_empty_identity!(ProjectName, "project");
non_empty_identity!(Slug, "slug");
non_empty_identity!(SkillName, "skill name");
non_empty_identity!(RepositoryName, "repository name");

#[derive(Debug, thiserror::Error)]
#[error("{field} is empty")]
struct InvalidIdentityField {
    field: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum DatasourceKind {
    Git,
    Svn,
    Confluence,
    Jira,
    Xray,
    File,
    Google,
    AzureDevopsWiki,
    AzureDevopsWorkItem,
    Sharepoint,
}

impl DatasourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Git => "git",
            Self::Svn => "svn",
            Self::Confluence => "confluence",
            Self::Jira => "jira",
            Self::Xray => "xray",
            Self::File => "file",
            Self::Google => "google",
            Self::AzureDevopsWiki => "azure_devops_wiki",
            Self::AzureDevopsWorkItem => "azure_devops_work_item",
            Self::Sharepoint => "sharepoint",
        }
    }
}

impl FromStr for DatasourceKind {
    type Err = InvalidDatasourceKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "git" => Ok(Self::Git),
            "svn" => Ok(Self::Svn),
            "confluence" => Ok(Self::Confluence),
            "jira" => Ok(Self::Jira),
            "xray" => Ok(Self::Xray),
            "file" => Ok(Self::File),
            "google" => Ok(Self::Google),
            "azure_devops_wiki" => Ok(Self::AzureDevopsWiki),
            "azure_devops_work_item" => Ok(Self::AzureDevopsWorkItem),
            "sharepoint" => Ok(Self::Sharepoint),
            _ => Err(InvalidDatasourceKind),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("datasource kind is unsupported")]
struct InvalidDatasourceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NaturalIdentityValue {
    Assistant {
        project: ProjectName,
        slug: Slug,
    },
    Workflow {
        project: ProjectName,
        slug: Slug,
    },
    Skill {
        project: ProjectName,
        name: SkillName,
    },
    Datasource {
        project: ProjectName,
        repo_name: RepositoryName,
        index_type: DatasourceKind,
    },
}

/// Validated natural identity of one declaration.
///
/// The representation is private so entity kind and selector shape cannot
/// disagree after boundary conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NaturalIdentity(NaturalIdentityValue);

impl NaturalIdentity {
    pub(crate) fn kind(&self) -> EntityKind {
        match self.0 {
            NaturalIdentityValue::Assistant { .. } => EntityKind::Assistant,
            NaturalIdentityValue::Workflow { .. } => EntityKind::Workflow,
            NaturalIdentityValue::Skill { .. } => EntityKind::Skill,
            NaturalIdentityValue::Datasource { .. } => EntityKind::Datasource,
        }
    }

    pub(crate) fn project(&self) -> &str {
        match &self.0 {
            NaturalIdentityValue::Assistant { project, .. }
            | NaturalIdentityValue::Workflow { project, .. }
            | NaturalIdentityValue::Skill { project, .. }
            | NaturalIdentityValue::Datasource { project, .. } => project.as_ref(),
        }
    }

    pub(crate) fn selector(&self) -> &str {
        match &self.0 {
            NaturalIdentityValue::Assistant { slug, .. }
            | NaturalIdentityValue::Workflow { slug, .. } => slug.as_ref(),
            NaturalIdentityValue::Skill { name, .. } => name.as_ref(),
            NaturalIdentityValue::Datasource { repo_name, .. } => repo_name.as_ref(),
        }
    }

    pub(crate) fn datasource_index_type(&self) -> Option<&str> {
        match &self.0 {
            NaturalIdentityValue::Datasource { index_type, .. } => Some(index_type.as_str()),
            _ => None,
        }
    }
}

impl TryFrom<&ParsedDeclaration> for NaturalIdentity {
    type Error = AppError;

    fn try_from(declaration: &ParsedDeclaration) -> Result<Self, Self::Error> {
        let value = match declaration.natural_identity()? {
            ParsedNaturalIdentity::Assistant { project, slug } => NaturalIdentityValue::Assistant {
                project: identity_field(ProjectName::try_from(project.to_owned()))?,
                slug: identity_field(Slug::try_from(slug.to_owned()))?,
            },
            ParsedNaturalIdentity::Workflow { project, slug } => NaturalIdentityValue::Workflow {
                project: identity_field(ProjectName::try_from(project.to_owned()))?,
                slug: identity_field(Slug::try_from(slug.to_owned()))?,
            },
            ParsedNaturalIdentity::Skill { project, name } => NaturalIdentityValue::Skill {
                project: identity_field(ProjectName::try_from(project.to_owned()))?,
                name: identity_field(SkillName::try_from(name.to_owned()))?,
            },
            ParsedNaturalIdentity::Datasource {
                project,
                repository,
                index_type,
            } => NaturalIdentityValue::Datasource {
                project: identity_field(ProjectName::try_from(project.to_owned()))?,
                repo_name: identity_field(RepositoryName::try_from(repository.to_owned()))?,
                index_type: index_type
                    .parse()
                    .map_err(|error: InvalidDatasourceKind| AppError::Schema(error.to_string()))?,
            },
        };
        Ok(Self(value))
    }
}

fn identity_field<T>(result: Result<T, InvalidIdentityField>) -> Result<T, AppError> {
    result.map_err(|error| AppError::Schema(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_id_accepts_uuid_and_rejects_arbitrary_text() {
        assert!(
            "123e4567-e89b-12d3-a456-426614174000"
                .parse::<WorkflowId>()
                .is_ok()
        );
        assert!("workflow-id".parse::<WorkflowId>().is_err());
    }

    #[test]
    fn natural_identity_keeps_kind_and_selector_shape_together() {
        let declaration = ParsedDeclaration::fixture(
            EntityKind::Datasource,
            serde_json::json!({
                "metadata": {"project": "demo", "repo_name": "docs"},
                "spec": {"index_type": "git"}
            }),
            "datasource.yaml",
        );
        let identity = NaturalIdentity::try_from(&declaration).unwrap();
        assert_eq!(identity.kind(), EntityKind::Datasource);
        assert_eq!(identity.project(), "demo");
        assert_eq!(identity.selector(), "docs");
        assert_eq!(identity.datasource_index_type(), Some("git"));
    }

    #[test]
    fn natural_identity_rejects_empty_semantic_fields() {
        let declaration = ParsedDeclaration::fixture(
            EntityKind::Assistant,
            serde_json::json!({
                "metadata": {"project": "", "slug": "assistant"},
                "spec": {}
            }),
            "assistant.yaml",
        );
        assert!(NaturalIdentity::try_from(&declaration).is_err());
    }

    #[test]
    fn natural_identity_rejects_unknown_datasource_kind() {
        let declaration = ParsedDeclaration::fixture(
            EntityKind::Datasource,
            serde_json::json!({
                "metadata": {"project": "demo", "repo_name": "docs"},
                "spec": {"index_type": "future-kind"}
            }),
            "datasource.yaml",
        );
        assert!(NaturalIdentity::try_from(&declaration).is_err());
    }
}
