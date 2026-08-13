# Data model: save server entity v3

## 1. Ownership and lifetime

| Data | Owner | Lifetime |
|---|---|---|
| Selector/project/output path | Save command | Invocation |
| Strict server snapshots | Kind read adapter | Invocation |
| Recovered natural references | Reverse projector | Invocation |
| Canonical YAML bytes | Serializer | Until direct write completes/fails |
| Final YAML path | Operator/Git workspace | Persistent, including partial failure |

No repository view, sidecar artifact, staging entry, temporary file, or
publication transaction exists.

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

Skill main content is part of the stable snapshot and maps directly to
`spec.content`. Companion payloads admitted by the schema remain inline
declaration values; no filesystem companion is generated.

The current pinned read contract cannot recover the original local File
Datasource path/source bytes, so that branch remains non-exportable. Save never
invents a path, inline encoding, or placeholder.

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
  | FinalCreated
  | Completed
  | FailedPartial
```

The writer transitions `NotStarted -> FinalCreated` only after all remote and
validation gates pass. `FinalCreated -> Completed` after all canonical bytes
are written and the file handle is successfully finalized. Any failure in
between becomes `FailedPartial`, returns `E_OUTPUT_WRITE`, and leaves the path
untouched by cleanup. No atomicity or durability beyond the ordinary file API
is claimed.

## 6. Invariants

1. Save sends no modifying HTTP request.
2. An existing target is never replaced or truncated.
3. Exactly one YAML file is the intentional output.
4. Success implies `Completed`; `FailedPartial` can never render `saved`.
5. All server reads/projection/confidentiality/validation precede final create.
6. No output path/content/server ID/raw error is emitted.
