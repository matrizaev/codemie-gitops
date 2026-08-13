use std::path::PathBuf;

pub(crate) mod publication;
mod reverse;
mod snapshot;

use publication::{publish, validate_output_path};
use reverse::{canonical_yaml, project_response};
use snapshot::{read_assistant, read_datasource, read_skill_snapshot, read_workflow};

use crate::domain::InputFile;
use crate::error::AppError;
use crate::http::{ApiClient, encode_query_value};
use crate::output::Outcome;
use base64::Engine;

const WORKFLOW_RESERVED_KEY: &str = "codemie.epam.com/gitops/workflow-identity";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveKind {
    Assistant,
    Workflow,
    Skill,
    Datasource,
}

impl std::str::FromStr for SaveKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Assistant" => Ok(Self::Assistant),
            "Workflow" => Ok(Self::Workflow),
            "Skill" => Ok(Self::Skill),
            "Datasource" => Ok(Self::Datasource),
            _ => Err("kind must be Assistant, Workflow, Skill, or Datasource".into()),
        }
    }
}

/// Untrusted command-line representation of a save request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSaveCommand {
    pub kind: SaveKind,
    pub project: String,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub repo_name: Option<String>,
    pub workflow_id: Option<String>,
    pub file: PathBuf,
    pub url: Option<String>,
}

/// Validated save command. Its selector and kind invariants are established at
/// the CLI boundary through [`TryFrom`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCommand {
    selector: SaveSelector,
    project: String,
    file: InputFile,
    url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SaveSelector {
    Assistant {
        slug: String,
    },
    Workflow {
        slug: String,
        workflow_id: Option<crate::domain::ServerId>,
    },
    Skill {
        name: String,
    },
    Datasource {
        repo_name: String,
    },
}

impl SaveSelector {
    fn kind(&self) -> SaveKind {
        match self {
            Self::Assistant { .. } => SaveKind::Assistant,
            Self::Workflow { .. } => SaveKind::Workflow,
            Self::Skill { .. } => SaveKind::Skill,
            Self::Datasource { .. } => SaveKind::Datasource,
        }
    }

    fn slug(&self) -> Option<&str> {
        match self {
            Self::Assistant { slug } | Self::Workflow { slug, .. } => Some(slug),
            Self::Skill { .. } | Self::Datasource { .. } => None,
        }
    }

    fn name(&self) -> Option<&str> {
        match self {
            Self::Skill { name } => Some(name),
            _ => None,
        }
    }

    fn repo_name(&self) -> Option<&str> {
        match self {
            Self::Datasource { repo_name } => Some(repo_name),
            _ => None,
        }
    }

    fn workflow_id(&self) -> Option<&str> {
        match self {
            Self::Workflow {
                workflow_id: Some(workflow_id),
                ..
            } => Some(workflow_id.as_str()),
            _ => None,
        }
    }
}

impl TryFrom<RawSaveCommand> for SaveCommand {
    type Error = AppError;

    fn try_from(raw: RawSaveCommand) -> Result<Self, Self::Error> {
        let selector_count =
            raw.slug.is_some() as u8 + raw.name.is_some() as u8 + raw.repo_name.is_some() as u8;
        let selector_is_valid = match raw.kind {
            SaveKind::Assistant | SaveKind::Workflow => raw.slug.is_some(),
            SaveKind::Skill => raw.name.is_some(),
            SaveKind::Datasource => raw.repo_name.is_some(),
        };
        if selector_count != 1 || !selector_is_valid {
            return Err(AppError::Usage("selector does not match save kind".into()));
        }
        if raw.workflow_id.is_some() && raw.kind != SaveKind::Workflow {
            return Err(AppError::Usage("--id is valid only for Workflow".into()));
        }
        let file = InputFile::try_from(raw.file)
            .map_err(|_| AppError::Usage("--file is required".into()))?;
        if raw.project.is_empty() {
            return Err(AppError::Usage("--project is required".into()));
        }
        let selector = match raw.kind {
            SaveKind::Assistant => SaveSelector::Assistant {
                slug: raw
                    .slug
                    .ok_or_else(|| AppError::Usage("slug is required".into()))?,
            },
            SaveKind::Workflow => SaveSelector::Workflow {
                slug: raw
                    .slug
                    .ok_or_else(|| AppError::Usage("slug is required".into()))?,
                workflow_id: raw
                    .workflow_id
                    .map(crate::domain::ServerId::try_from)
                    .transpose()
                    .map_err(|_| AppError::Usage("--id cannot be empty".into()))?,
            },
            SaveKind::Skill => SaveSelector::Skill {
                name: raw
                    .name
                    .ok_or_else(|| AppError::Usage("name is required".into()))?,
            },
            SaveKind::Datasource => SaveSelector::Datasource {
                repo_name: raw
                    .repo_name
                    .ok_or_else(|| AppError::Usage("repo-name is required".into()))?,
            },
        };
        Ok(Self {
            selector,
            project: raw.project,
            file,
            url: raw.url,
        })
    }
}

impl SaveCommand {
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    pub fn with_resolved_url(mut self, url: String) -> Result<Self, AppError> {
        self.url = Some(url);
        Ok(self)
    }
}

pub async fn save(command: SaveCommand) -> Result<Outcome, AppError> {
    tokio::time::timeout(std::time::Duration::from_secs(300), save_inner(command))
        .await
        .map_err(|_| AppError::Timeout("save exceeded the 300-second deadline".into()))?
}

async fn save_inner(command: SaveCommand) -> Result<Outcome, AppError> {
    let url = command
        .url
        .as_deref()
        .ok_or_else(|| AppError::Configuration("target URL is required for save".into()))?;
    let validated_url = crate::config::ValidatedUrl::try_from(url)?;
    let output_path = validate_output_path(command.file.as_path())?;
    let client = ApiClient::new(
        validated_url.clone(),
        std::env::var("CODEMIE_TOKEN").unwrap_or_default().into(),
    )?;
    let project = command.project.as_str();
    let response = if command.selector.kind() == SaveKind::Skill {
        read_skill_snapshot(
            &client,
            &validated_url,
            project,
            command
                .selector
                .name()
                .ok_or_else(|| AppError::Usage("name is required".into()))?,
        )
        .await?
    } else if command.selector.kind() == SaveKind::Datasource {
        read_datasource(
            &client,
            &validated_url,
            project,
            command
                .selector
                .repo_name()
                .ok_or_else(|| AppError::Usage("repo-name is required".into()))?,
        )
        .await?
    } else if command.selector.kind() == SaveKind::Workflow {
        read_workflow(
            &client,
            &validated_url,
            project,
            command
                .selector
                .slug()
                .ok_or_else(|| AppError::Usage("slug is required".into()))?,
            command.selector.workflow_id(),
        )
        .await?
    } else {
        read_assistant(
            &client,
            project,
            command
                .selector
                .slug()
                .ok_or_else(|| AppError::Usage("slug is required".into()))?,
        )
        .await?
    };
    let (declaration, adoption_required) = project_response(&command, project, response)?;
    let yaml = canonical_yaml(&declaration)?;
    crate::input::validate_generated(&yaml)?;
    publish(&output_path, yaml.as_bytes())?;
    outcome(&command, project, adoption_required)
}

fn outcome(
    command: &SaveCommand,
    project: &str,
    adoption_required: bool,
) -> Result<Outcome, AppError> {
    Ok(match &command.selector {
        SaveSelector::Assistant { slug } => {
            Outcome::assistant(crate::output::Action::Saved, project.into(), slug.clone())
        }
        SaveSelector::Workflow { slug, .. } => {
            Outcome::saved_workflow(project.into(), slug.clone(), adoption_required)
        }
        SaveSelector::Skill { name } => {
            Outcome::new_skill(crate::output::Action::Saved, project.into(), name.clone())
        }
        SaveSelector::Datasource { repo_name } => Outcome::new_datasource(
            crate::output::Action::Saved,
            project.into(),
            repo_name.clone(),
        ),
    })
}
