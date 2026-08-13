# Data model: CodeMie declarative CLI v33

## 1. Ownership and lifetime

| Data | Owner | Lifetime |
|---|---|---|
| Selected YAML bytes | Single declaration loader | Current invocation only |
| Validated declaration | Domain/application layer | Current invocation only |
| Natural references | Declaration domain | Current invocation only |
| Resolved server IDs/capabilities | Kind adapter | Current apply only |
| Credentials | Auth boundary | Process/request memory only |
| Outcome/diagnostic | Output boundary | One rendered record |

There is no repository model, repository index, declaration closure, implicit
input discovery, or persisted ID map. Explicit auxiliary Skill/File inputs are
invocation-only bounded reads.

## 2. Boundary types

```text
RawLintCommand  { file: PathBuf, output: OutputMode }
RawApplyCommand { file: PathBuf, url?: String, adopt_workflow_id?: String,
                  output: OutputMode }

InputFile              = validated non-empty path selector
BoundedDeclaration     = bytes within the declaration byte budget
MarkedDocument         = one duplicate-key-safe YAML document with locations
ValidatedDeclaration   = closed v1alpha1 schema + semantic invariants
EffectiveDeclaration   = validated declaration with explicit project
NaturalReference       = { kind, project, natural_key }
ResolvedReference      = invocation-local server representation
```

Project is authoring-required in v33 declarations. No filesystem configuration
can materialize it.

## 3. Validation boundary

The loader performs one ordinary declaration open/read. It rejects a non-regular,
oversized, non-UTF-8, multi-document, duplicate-key, alias-budget-exceeding, or
schema-invalid declaration. After schema validation it may open only the exact
Skill `contentFrom` or File Datasource `files[]` paths authored by that
declaration through the validated auxiliary-input boundary.

Offline semantic validation covers kind rules, field relationships, natural
reference shapes, and Workflow-local actor/state IDs. It does not build a
cross-file symbol table. Apply adapters resolve all server asset references
before mutation.

## 4. Request and consistency model

```text
ApplyPlan = {
  desired: EffectiveDeclaration,
  target: Missing | Existing { server_id, write_capability },
  references: list<ResolvedReference>,
  operation: Create | Update
}
```

The plan is immutable after resolution. A successful valid apply sends exactly
one modifying request and then verifies the natural identity as required by the
kind adapter. Local validation is read-only; remote reads and the write are not
a distributed transaction.

```text
ExplicitAuxiliaryInput = SkillMarkdownPath | DatasourceFilePath
```

Paths are nonempty relative values resolved from the declaration parent. Their
validated target is contained, non-symlink, regular, readable, and unique where
multiple File inputs exist. Skill is UTF-8 and bounded to 131,072 bytes plus
100–30,000 content characters. File inputs are 1–10, at most 32 MiB each and
128 MiB aggregate. Apply retains bytes only for inline Skill projection or
multipart streaming; no temporary/staging copy exists.

## 5. Configuration

```text
TargetUrl = --url | CODEMIE_URL
AuthUrl   = --auth-url | CODEMIE_AUTH_URL
Token     = CODEMIE_TOKEN
ClientSecret = CODEMIE_CLIENT_SECRET
Password  = CODEMIE_PASSWORD
```

Invalid higher-precedence values fail closed. There is no configuration-file
source and no repository-root discovery.

## 6. Time and resource bounds

- one YAML input: at most the approved declaration byte budget;
- marked parser depth/alias/scalar limits remain enforced;
- invocation deadline: 300 seconds;
- HTTP request timeout: 60 seconds;
- existing response, pagination, upload, and concurrency bounds remain.

The timeout is application policy. Domain types and parsing interfaces do not
require a public cancellation-token parameter.

## 7. Failure model

Failures remain typed internally and cross the external boundary only as the
closed diagnostic schema. A local read/parse/schema failure is exit 2; safe
reconciliation conflicts remain exit 1; success is exit 0. Paths, declaration
content, raw server data, credentials, and arbitrary exception text are not
diagnostic fields.
