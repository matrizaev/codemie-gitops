use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::AppError;
use crate::parse::{EntityKind, ParsedDeclaration, ParsedNaturalIdentity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InputFile(PathBuf);

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
///
/// Stores the canonical hyphenated lowercase spelling so the string
/// representation is itself the invariant; simple, braced, URN, and uppercase
/// forms are rejected at conversion.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct WorkflowId(String);

impl WorkflowId {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for WorkflowId {
    type Err = InvalidWorkflowId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parsed = uuid::Uuid::parse_str(value).map_err(|_| InvalidWorkflowId)?;
        if parsed.hyphenated().to_string() != value {
            return Err(InvalidWorkflowId);
        }
        Ok(Self(value.to_owned()))
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("workflow ID is not a canonical hyphenated UUID")]
pub(crate) struct InvalidWorkflowId;

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

/// Schema-matching identifier constraints for the validated identity newtypes.
///
/// These mirror the closed declaration schema's metadata-field rules so that
/// invalid identifiers are unrepresentable after boundary conversion, not just
/// after schema validation. Lengths count Unicode scalar values, matching the
/// schema's maxLength semantics.
mod constraints {
    /// C0/C1 controls and the Unicode bidi controls excluded by the schema.
    pub(super) fn has_no_controls(value: &str) -> bool {
        value.chars().all(|character| {
            let codepoint = character as u32;
            !(codepoint <= 0x1f
                || (0x7f..=0x9f).contains(&codepoint)
                || (0x202a..=0x202e).contains(&codepoint)
                || (0x2066..=0x2069).contains(&codepoint))
        })
    }

    /// metadata.project: 1-100 characters, no controls or bidi controls.
    pub(super) fn project(value: &str) -> bool {
        (1..=100).contains(&value.chars().count()) && has_no_controls(value)
    }

    /// metadata.slug (Assistant/Workflow): 1-100 characters, same exclusions.
    pub(super) fn slug(value: &str) -> bool {
        (1..=100).contains(&value.chars().count()) && has_no_controls(value)
    }

    /// metadata.name (Skill): 3-64 bytes, pattern [a-z0-9][a-z0-9-]{1,62}[a-z0-9].
    pub(super) fn skill_name(value: &str) -> bool {
        let bytes = value.as_bytes();
        let len = bytes.len();
        (3..=64).contains(&len)
            && bytes
                .first()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            && bytes
                .last()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == 45)
    }

    /// metadata.repo_name (Datasource): 4-50 bytes, starts alphanumeric,
    /// then alphanumeric, underscore, or hyphen.
    pub(super) fn repository_name(value: &str) -> bool {
        let bytes = value.as_bytes();
        let len = bytes.len();
        (4..=50).contains(&len)
            && bytes
                .first()
                .is_some_and(|byte| byte.is_ascii_alphanumeric())
            && bytes
                .iter()
                .all(|byte| byte.is_ascii_alphanumeric() || *byte == 95 || *byte == 45)
    }
}

macro_rules! identity_newtype {
    ($name:ident, $label:literal, $validate:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        struct $name(String);

        impl TryFrom<String> for $name {
            type Error = InvalidIdentityField;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if $validate(&value) {
                    Ok(Self(value))
                } else {
                    Err(InvalidIdentityField { field: $label })
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

identity_newtype!(ProjectName, "project", constraints::project);
identity_newtype!(Slug, "slug", constraints::slug);
identity_newtype!(SkillName, "skill name", constraints::skill_name);
identity_newtype!(
    RepositoryName,
    "repository name",
    constraints::repository_name
);

#[derive(Debug, thiserror::Error)]
#[error("{field} is invalid")]
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

    /// Map a completed apply action to the closed per-kind success outcome.
    pub(crate) fn success_outcome(
        &self,
        action: crate::adapters::ApplyAction,
    ) -> crate::output::Outcome {
        let action = match action {
            crate::adapters::ApplyAction::Created => crate::output::Action::Created,
            crate::adapters::ApplyAction::Updated => crate::output::Action::Updated,
        };
        match self.kind() {
            EntityKind::Assistant => crate::output::Outcome::assistant(
                action,
                self.project().to_owned(),
                self.selector().to_owned(),
            ),
            EntityKind::Workflow => crate::output::Outcome::workflow(
                action,
                self.project().to_owned(),
                self.selector().to_owned(),
            ),
            EntityKind::Skill => crate::output::Outcome::new_skill(
                action,
                self.project().to_owned(),
                self.selector().to_owned(),
            ),
            EntityKind::Datasource => crate::output::Outcome::new_datasource(
                action,
                self.project().to_owned(),
                self.selector().to_owned(),
            ),
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
    fn workflow_id_rejects_non_canonical_spellings() {
        // Simple, braced, URN, and uppercase forms are not canonical.
        assert!(
            "123e4567e89b12d3a456426614174000"
                .parse::<WorkflowId>()
                .is_err()
        );
        assert!(
            "{123e4567-e89b-12d3-a456-426614174000}"
                .parse::<WorkflowId>()
                .is_err()
        );
        assert!(
            "urn:uuid:123e4567-e89b-12d3-a456-426614174000"
                .parse::<WorkflowId>()
                .is_err()
        );
        assert!(
            "123E4567-E89B-12D3-A456-426614174000"
                .parse::<WorkflowId>()
                .is_err()
        );
        // Round-trip keeps the canonical spelling.
        let id: WorkflowId = "123e4567-e89b-12d3-a456-426614174000".parse().unwrap();
        assert_eq!(id.as_str(), "123e4567-e89b-12d3-a456-426614174000");
    }

    #[test]
    fn identity_newtypes_enforce_schema_constraints() {
        // project: 1-100 chars, no controls/bidi.
        assert!(ProjectName::try_from("demo".to_owned()).is_ok());
        assert!(ProjectName::try_from(String::new()).is_err());
        let bidi = format!("a{}b", '\u{202e}');
        assert!(ProjectName::try_from(bidi).is_err());
        assert!(ProjectName::try_from("x".repeat(101)).is_err());
        // slug: 1-100 chars, no controls/bidi (spaces permitted by the schema).
        assert!(Slug::try_from("my assistant".to_owned()).is_ok());
        assert!(
            Slug::try_from(
                "bad
slug"
                    .to_owned()
            )
            .is_err()
        );
        assert!(Slug::try_from("x".repeat(101)).is_err());
        // skill name: 3-64, [a-z0-9][a-z0-9-]{1,62}[a-z0-9].
        assert!(SkillName::try_from("triage-skill".to_owned()).is_ok());
        assert!(SkillName::try_from("ab".to_owned()).is_err());
        assert!(SkillName::try_from("UPPER".to_owned()).is_err());
        assert!(SkillName::try_from("ends-with-dash-".to_owned()).is_err());
        // repo_name: 4-50, starts alphanumeric, then alnum/_/-.
        assert!(RepositoryName::try_from("product-docs".to_owned()).is_ok());
        assert!(RepositoryName::try_from("abc".to_owned()).is_err());
        assert!(RepositoryName::try_from("bad name".to_owned()).is_err());
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
