# YAML declaration reference

This page documents the complete `codemie.epam.com/v1alpha1` authoring format. The [JSON Schema](../specs/codemie-cicd-tool/contracts/declaration-v1alpha1.schema.json) is the machine-readable source of truth.

Each file contains exactly one declaration. Objects are closed unless this page explicitly calls them unrestricted: otherwise, keys not listed here are invalid. A field marked **required** must be present. “Optional, nullable” means it may be omitted or written as `null`; other optional fields must have the stated type when present. Empty arrays are valid unless a minimum is stated. YAML duplicate keys, multiple documents, custom tags, and server IDs are rejected.

```yaml
apiVersion: codemie.epam.com/v1alpha1 # required, exact value
kind: Assistant                       # Assistant | Workflow | Skill | Datasource
metadata: {}
spec: {}
```

`metadata.project` is required for every kind: a string of 1–100 characters excluding control characters and Unicode bidi controls. Assistant and Workflow use required `metadata.slug` (1–100 characters with the same exclusions). Skill uses required `metadata.name` (3–64 characters, pattern `[a-z0-9][a-z0-9-]{1,62}[a-z0-9]`). Datasource uses required `metadata.repo_name` (4–50 characters, starts alphanumeric, then alphanumeric, `_`, or `-`).

## Shared nested objects

These objects are reused below.

| Object | Fields |
|---|---|
| Assistant reference | `project` and `slug`: required non-empty strings (`project` uses the project constraint above). |
| Skill reference | `project` and `name`: required; `name` uses the Skill name constraint above. |
| Datasource reference | `project` required; `repo_name` required string, 1–50 characters. |
| Integration selection | `id`: required non-empty string; `alias`: optional nullable string. |
| Guardrail assignment | `guardrail_id`: required non-empty string; `mode`: required `all` or `filtered`; `source`: required `input`, `output`, or `both`; `editable`, `guardrail_name`: optional nullable boolean/string. |
| Tool | `name`: required non-empty string; `settings_config`: required boolean; `label`, `description`, `user_description`: optional nullable strings; `settings`: optional nullable integration selection. |
| Toolkit | `toolkit`: required non-empty string; `tools`: required array of Tool; `label`: required string; `settings_config`: required boolean; `settings`: optional nullable integration selection; `is_external`: optional nullable boolean. |
| MCP server | `name`: required non-empty string; `enabled`, `use_custom_config`, `resolve_dynamic_values_in_arguments`: required booleans; `description`, `mcp_config_id`, `command`, `arguments`, `integration_alias`: optional nullable strings; `mcp_connect_url`: optional nullable HTTP(S) URL without credentials, fragment, whitespace, or control characters; `tools_tokens_size_limit`: optional nullable integer ≥1; `settings`: optional nullable integration selection; `tools`: optional nullable array of strings. |
| Prompt variable | `key`: required string, 1–100 characters; `default_value`: required string, at most 500 characters; `is_sensitive`: required boolean; `description`: optional nullable string, at most 200 characters. |
| Interactive features | `action_buttons`, `choice`, `short_forms`: all required booleans. |

### Hedging configuration

`timeout_ms` (integer ≥1) and `input_mapping` (object whose values are strings) are required. Exactly one of `tool` and `provider_tool` is required and must be non-null. `tool` contains only required non-empty `name`. `provider_tool` requires non-empty `provider_name`, `toolkit_name`, and `tool_name`, with optional nullable `datasource_name` and `result_condition`. `output_field` is an optional nullable string.

## Assistant

### Fields

| Field | Presence and constraints |
|---|---|
| `name` | Required non-empty string. |
| `system_prompt` | Required non-empty string. |
| `llm_model_type` | Required non-empty string. |
| `type` | Required; exact value `codemie`. |
| `context` | Required array of `{context_type, ref}`. `context_type` is `knowledge_base`, `code`, or `provider`; `ref` is a Datasource reference. |
| `toolkits` | Required array of Toolkit. |
| `conversation_starters` | Required array of strings. |
| `shared` | Required boolean. |
| `mcp_servers` | Required array of MCP server. |
| `sub_assistants` | Required array of Assistant references. |
| `enabled_builtin_subagents` | Required unique array; its only admitted item is `GENERAL_PURPOSE_SUBAGENT`. |
| `skills` | Required array of Skill references. |
| `categories` | Required array of category ID slugs (e.g. `data-analytics`), at most 3 items. Categories must already exist on the server. |
| `prompt_variables` | Required array of Prompt variable. |
| `description`, `icon_url`, `image_generation_model`, `plan_prompt` | Optional nullable strings. |
| `enable_image_generation`, `is_global`, `smart_tool_selection_enabled` | Optional nullable booleans. |
| `agent_mode` | Optional: `general`, `plan_execute`, or null. If `plan_execute`, `plan_prompt` is required and must be a non-empty string. |
| `temperature` | Optional nullable number, 0–2 inclusive. |
| `top_p` | Optional nullable number, 0–1 inclusive. |
| `tools_tokens_size_limit` | Optional nullable integer ≥1. |
| `hedging_config` | Optional nullable Hedging configuration. |
| `interactive_features` | Optional nullable Interactive features. |
| `custom_metadata` | Optional nullable object; its contents are unrestricted by the schema. |
| `guardrail_assignments` | Optional nullable array of Guardrail assignment. |

### Complete minimal example

```yaml
apiVersion: codemie.epam.com/v1alpha1
kind: Assistant
metadata:
  project: demo
  slug: support-assistant
spec:
  name: Support assistant
  system_prompt: Help the user.
  llm_model_type: gpt-model
  type: codemie
  context: []
  toolkits: []
  conversation_starters: []
  shared: false
  mcp_servers: []
  sub_assistants: []
  enabled_builtin_subagents: []
  skills: []
  categories: []
  prompt_variables: []
```

## Skill

| Field | Presence and constraints |
|---|---|
| `description` | Required string, 10–1000 characters. |
| `content` / `contentFrom` | Exactly one is required. `content` is 100–30000 characters. `contentFrom` is a non-empty relative path ending `.md`; absolute POSIX and Windows drive paths are rejected. Runtime path rules are described below. |
| `visibility` | Required: `private`, `project`, or `public`. |
| `categories` | Required array of category ID slugs (e.g. `data-analytics`), at most 3 items. Categories must already exist on the server. |
| `toolkits` | Required array of Toolkit. |
| `mcp_servers` | Required array of MCP server. |
| `companion_files` | Required array of companion files. Each requires non-empty `path`, non-empty `mime_type`, `encoding` (`text` or `base64`), integer `size_bytes` ≥0, and string `content`. |
| `enabled_builtin_subagents` | Required unique array; its only admitted item is `GENERAL_PURPOSE_SUBAGENT`. |

`contentFrom` resolves relative to the declaration file. It must identify one `.md` regular file without escaping the declaration directory or traversing symlinks. The resolved content is subject to the same 100–30000 character bound and is sent inline; paths/content are not placed in diagnostics.

```yaml
apiVersion: codemie.epam.com/v1alpha1
kind: Skill
metadata:
  project: demo
  name: triage-skill
spec:
  description: Triage incoming support requests consistently.
  content: |-
    Review the incoming request, identify its urgency and affected component, and produce a concise triage summary. Include the evidence used, the recommended owner, and one practical next action for the support team.
  visibility: project
  categories: [support]
  toolkits: []
  mcp_servers: []
  companion_files: []
  enabled_builtin_subagents: []
```

## Workflow

### Workflow spec

| Field | Presence and constraints |
|---|---|
| `name` | Required non-empty string. |
| `description` | Required string (may be empty). |
| `mode` | Required: `Sequential` or `Autonomous`. `Autonomous` additionally requires non-empty `supervisor_prompt`. |
| `execution_config` | Required Execution configuration. |
| `shared` | Required boolean. |
| `start_hint`, `icon_url`, `supervisor_prompt` | Optional nullable strings, subject to the mode rule above. |
| `meta_config` | Optional nullable object. The key `codemie.epam.com/gitops/workflow-identity` is forbidden; all other contents are unrestricted by the schema. |
| `guardrail_assignments` | Optional nullable array of Guardrail assignment. |

### Execution configuration and actors

Execution configuration requires: `messages_limit_before_summarization` (integer 1–10000), `tokens_limit_before_summarization` (integer 1–2147483647), `type` (string), `enable_summarization_node` (boolean), `recursion_limit` (integer 1–5000), `max_concurrency` (integer 1–100), `verbose` (boolean), `max_iteration_key_output_limit` (integer 1–10000), `assistants` (actor array), `tools` (Workflow tool array), `custom_nodes` (custom-node array), `states` (state array), and `retry_policy` (Retry policy).

Each actor is exactly one of:

- Persisted actor: required `id` (non-empty), `assistantRef` (Assistant reference), `name` and `model` (strings), `limit_tool_output_tokens` (integer 1–2147483647), `tools` (Workflow assistant-tool array), `exclude_extra_context_tools` (boolean), and `mcp_servers` (MCP server array); optional nullable `temperature` (0–2).
- Inline actor: the same required fields except it has no `assistantRef`; it additionally requires non-empty `system_prompt`, `skillRefs` (Skill reference array), and `datasourceRefs` (Datasource reference array). `name` and `model` are strings and may be empty.

A Workflow assistant tool requires non-empty `name`; optional `integration_alias` is a non-null string.

### Workflow tools, custom nodes, retry, states, and transitions

| Object | Fields and rules |
|---|---|
| Workflow tool | Required: non-empty `id`, non-empty `tool`, object `tool_args`, string `integration_alias`, boolean `trace`, boolean `resolve_dynamic_values_in_response`, string `input_key`. Optional: nullable string `tool_result_json_pointer`; nullable MCP server `mcp_server`; nullable integer ≥1 `tokens_size_limit`. |
| Custom node | Required: non-empty `id`, non-empty `custom_node_id`, string `name`, string `model`, string `system_prompt`, and closed `config`. Config optionally admits strings `documents_filter`, `documents_filtering_pattern`, `output_template` and string array `states_status_filter`; the two document-filter fields cannot coexist. |
| Retry policy | Required numbers `initial_interval` and `max_interval` (0–3600000), number `backoff_factor` (1–10), and integer `max_attempts` (1–100). |
| State | Required: non-empty `id`, string `task`, boolean `finish_iteration`, Transition `next`, Retry policy `retry_policy`, booleans `interrupt_before`, `resolve_dynamic_values_in_prompt`, and `result_as_human_message`. Exactly one target is required: non-empty `assistant_id`; non-empty `custom_node_id`; or non-empty `tool_id` together with object `tool_args`. Optional `output_schema` is a string. |
| Transition | Required booleans `override_task`, `store_in_context`, `include_in_llm_history`, `clear_prior_messages`, `append_to_context`; required `clear_context_store` is boolean or `keep_current`; required `include_in_iterator_context` is a string array. Optional strings: `state_id`, `iter_key`, `output_key`; optional string arrays: `state_ids`, `reset_keys_in_context_store`; optional Condition or Switch. |
| Condition | Required string `expression`, `then`, and `otherwise`. |
| Switch | Required `cases` array and string `default`; each case requires string `condition` and `state_id`. |

Transition exclusions: `condition` and `switch` cannot coexist; `state_id` and `state_ids` cannot coexist; `state_ids` cannot coexist with `iter_key`, `condition`, or `switch`; `iter_key` cannot coexist with `condition` or `switch`, and requires `state_id`.

```yaml
apiVersion: codemie.epam.com/v1alpha1
kind: Workflow
metadata:
  project: demo
  slug: support-flow
spec:
  name: Support flow
  description: Route one request.
  mode: Sequential
  shared: false
  execution_config:
    messages_limit_before_summarization: 20
    tokens_limit_before_summarization: 4000
    type: sequential
    enable_summarization_node: false
    recursion_limit: 20
    max_concurrency: 1
    verbose: false
    max_iteration_key_output_limit: 100
    assistants:
      - id: triage
        name: Triage
        model: gpt-model
        system_prompt: Triage the request.
        limit_tool_output_tokens: 1000
        tools: []
        exclude_extra_context_tools: false
        mcp_servers: []
        skillRefs: []
        datasourceRefs: []
    tools: []
    custom_nodes: []
    states:
      - id: triage-state
        assistant_id: triage
        task: Triage the request.
        finish_iteration: true
        next:
          override_task: false
          store_in_context: false
          include_in_llm_history: true
          clear_prior_messages: false
          clear_context_store: false
          include_in_iterator_context: []
          append_to_context: false
        retry_policy:
          initial_interval: 1
          backoff_factor: 2
          max_interval: 30
          max_attempts: 3
        interrupt_before: false
        resolve_dynamic_values_in_prompt: false
        result_as_human_message: true
    retry_policy:
      initial_interval: 1
      backoff_factor: 2
      max_interval: 30
      max_attempts: 3
```

## Datasource

All variants use the Datasource envelope and are selected by `spec.index_type`. Every variant requires `description` (1–500 characters). Unless stated otherwise, `project_space_visible`, `embedding_model`, `setting_id`, `cron_expression`, and `timezone` are optional nullable values of the evident scalar type, and `guardrail_assignments` is an optional nullable Guardrail assignment array. Field spelling is significant.

### Variant fields

| `index_type` | Required fields | Optional fields and constraints |
|---|---|---|
| `git` | `link`: HTTP(S) URL, 1–1000 chars, no credentials/fragment/whitespace/control; `branch`: 1–1000 chars, begins alphanumeric, then alphanumeric/`_`/`.`/`/`/`-`; `indexType`: `code`, `summary`, or `chunk-summary`; `projectSpaceVisible`: boolean | Nullable strings `filesFilter`, `embeddingsModel`, `summarizationModel`, `prompt`, `setting_id`, `cron_expression`, `timezone`; nullable boolean `docsGeneration`; guardrails. |
| `svn` | As `git`, except `link` also admits `svn://` and `svn+ssh://`. | Same optional fields as `git`. |
| `confluence` | Non-empty string `cql`. | Nullable booleans `project_space_visible`, `include_restricted_content`, `include_archived_content`, `include_attachments`, `include_comments`, `keep_markdown_format`, `keep_newlines`; nullable strings `setting_id`, `embedding_model`, `cron_expression`, `timezone`; guardrails. |
| `jira` | Non-empty string `jql`. | Nullable boolean `project_space_visible`; nullable strings `setting_id`, `embedding_model`, `cron_expression`, `timezone`; guardrails. |
| `xray` | Non-empty string `jql`. | Same optional fields as `jira`. |
| `azure_devops_wiki` | String `wiki_query` (may be empty). | Nullable boolean `project_space_visible`; nullable strings `wiki_name`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`; guardrails. |
| `azure_devops_work_item` | String `wiql_query` (may be empty). | Nullable boolean `project_space_visible`; nullable strings `setting_id`, `embedding_model`, `cron_expression`, `timezone`; guardrails. |
| `file` | `files`: 1–10 non-empty path strings; `include_email_attachments`: boolean. | Nullable boolean `project_space_visible`; nullable string arrays `uploaded_files` (each item non-empty); nullable strings `csv_separator`, `embedding_model`; nullable integer `csv_start_row` ≥0; nullable integer `csv_rows_per_document` ≥1; guardrails. |
| `google` | `googleDoc`: Google Docs edit URL matching `https://docs.google.com/document/(u/<digits>/)?d/<id>/edit` with optional query/fragment; non-empty string `setting_id`. | Nullable boolean `project_space_visible`; nullable strings `embedding_model`, `cron_expression`, `timezone`; guardrails. |
| `sharepoint` | `site_url`: HTTPS URL without credentials/fragment/whitespace/control; `auth_type`: `integration`, `oauth_codemie`, or `oauth_custom`. | Nullable booleans `project_space_visible`, `include_pages`, `include_documents`, `include_lists`; nullable integer `max_file_size_mb` 1–500; nullable strings `files_filter`, `setting_id`, `embedding_model`, `cron_expression`, `timezone`, `oauth_client_id`, `oauth_tenant_id`; guardrails. |

For `file`, each `files` item resolves relative to the declaration. Files must be distinct regular files, remain inside that directory, and not traverse symlinks. Limits are 32 MiB per file, 128 MiB total, and 10 files. `uploaded_files` records retained server filenames and is not a source of local bytes.

### Complete minimal examples for every variant

```yaml
# git
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: code-repo}
spec: {index_type: git, description: Source code, link: "https://example.com/repo.git", branch: main, indexType: code, projectSpaceVisible: true}
```

```yaml
# svn
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: svn-repo}
spec: {index_type: svn, description: SVN source, link: "svn://example.com/repo", branch: trunk, indexType: summary, projectSpaceVisible: false}
```

```yaml
# confluence
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: confluence-docs}
spec: {index_type: confluence, description: Product pages, cql: "space = DOCS"}
```

```yaml
# jira
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: jira-issues}
spec: {index_type: jira, description: Project issues, jql: "project = DEMO"}
```

```yaml
# xray
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: xray-tests}
spec: {index_type: xray, description: Test issues, jql: "project = DEMO"}
```

```yaml
# azure_devops_wiki
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: azure-wiki}
spec: {index_type: azure_devops_wiki, description: Engineering wiki, wiki_query: "path:/engineering"}
```

```yaml
# azure_devops_work_item
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: azure-items}
spec: {index_type: azure_devops_work_item, description: Work items, wiql_query: "SELECT [System.Id] FROM WorkItems"}
```

```yaml
# file (the referenced file must exist beside this declaration for CLI lint/apply)
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: local-files}
spec: {index_type: file, description: Local documents, files: [guide.pdf], include_email_attachments: false}
```

```yaml
# google
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: google-doc}
spec: {index_type: google, description: Design document, googleDoc: "https://docs.google.com/document/d/document-id/edit", setting_id: google-integration}
```

```yaml
# sharepoint
apiVersion: codemie.epam.com/v1alpha1
kind: Datasource
metadata: {project: demo, repo_name: sharepoint-docs}
spec: {index_type: sharepoint, description: Team site, site_url: "https://example.sharepoint.com/sites/team", auth_type: integration}
```

## Omission, nulls, and local validation

Required fields cannot be omitted or set to null unless their stated type explicitly admits null. Optional non-null fields may be omitted, but cannot be written as null. Optional nullable fields accept both omission and explicit `null`; when applicable, the adapter projects omitted/null optional fields as JSON null rather than discovering a server default.

Natural references are structural during offline lint: referenced objects need not exist in adjacent files. Apply resolves them against the server. Validate one declaration with:

```sh
codemie-gitops lint --file path/to/declaration.yaml
```
