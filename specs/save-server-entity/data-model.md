# Data model: save server entity v3.5

## 1. Ownership and lifetime

| Data | Owner | Lifetime |
|---|---|---|
| Selector/project/output path | Save command | Invocation |
| Strict server snapshots | Kind read adapter | Invocation |
| Recovered natural references | Reverse projector | Invocation |
| Canonical YAML bytes | Serializer | Until direct write completes/fails |
| Final YAML path | Operator/Git workspace | Persistent, including partial failure |
| File Datasource placeholder paths | Reverse projector/publisher | Invocation, then operator workspace |
| Zero-byte placeholder files | Operator/Git workspace | Persistent until populated or removed |

No repository view, staging entry, temporary file, or atomic publication
transaction exists. File Datasource placeholders are explicit declaration
inputs, not hidden content sidecars.

## 2. Validated command

```text
SaveCommand = {
  selector: AssistantSlug | WorkflowSlugAndOptionalId |
            SkillName | DatasourceRepoName,
  project: ProjectName,
  output: NewOutputPath,
  target_url: ValidatedUrl,
  output_mode: Text | Json
}
```

Project is explicit; no repository config fallback exists. Removed flags are
rejected at argument parsing.

## 3. Read and reverse model

Existing strict snapshot types remain. Response members are classified as
authorable, managed reference, read-only, secret/non-exportable, or unknown.
Unknown authoring-relevant, secret, masked, inconsistent, or unstable evidence
fails closed. Managed IDs are recovered to natural references before rendering.

OpenAPI response objects are not declaration objects. Reverse projection
normalizes API representations before validation:

- Assistant context `{context_type, name}` becomes `{context_type, ref}` with
  the explicit Assistant project and the datasource `repo_name`.
- Enriched Assistant category objects become category-name strings.
- Toolkit and tool objects retain only declaration fields; API-only metadata
  and integration settings metadata are excluded, while settings retain only
  `id` and `alias`.
- MCP server objects retain declaration fields, omit nested `config` and
  credential-bearing values, and materialize only defaults declared by the
  pinned API contract when required by the declaration schema.

The same normalization rules apply to Assistant and Skill toolkit/MCP fields.

Skill main content is part of the stable snapshot and maps directly to
`spec.content`. Companion payloads admitted by the schema remain inline
declaration values; no filesystem companion is generated.

For `knowledge_base_file`, the server exposes metadata and `uploaded_files` but
not source bytes. Reverse projection preserves `uploaded_files`, maps the kind
to `spec.index_type: file`, and derives up to ten safe relative paths beneath
`<yaml-name>.files/`. Each path receives a zero-byte placeholder. Unsafe or
duplicate basenames receive deterministic `replace-content-N.txt` names.

## 4. Generated declaration

```text
GeneratedDeclaration = {
  value: ValidatedDeclaration,
  canonical_yaml: ImmutableBytes
}
```

The same one-declaration schema/semantic validator used by lint validates the
generated value/bytes in memory. It opens no local files and performs no
reference-existence check.

## 5. Output state

```text
OutputWriteState =
  | NotStarted
  | PlaceholdersCreated
  | FinalCreated
  | Completed
  | FailedPartial
```

Ordinary kinds transition directly from `NotStarted` to `FinalCreated`. File
Datasource transitions through `PlaceholdersCreated`, then creates YAML last.
Any failure becomes `FailedPartial` and performs no cleanup. An orphan
placeholder directory may therefore remain without YAML. No atomicity or
durability beyond ordinary file APIs is claimed.

## 6. Invariants

1. Save sends no modifying HTTP request.
2. An existing target is never replaced or truncated.
3. Exactly one YAML is emitted; File Datasource may additionally emit only the
   placeholder files referenced by that YAML.
4. Success implies `Completed`; `FailedPartial` can never render `saved`.
5. All server reads/projection/confidentiality/validation precede final create.
6. No output path/content/server ID/raw error is emitted.
