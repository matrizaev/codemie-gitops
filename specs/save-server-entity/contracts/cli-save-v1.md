# Save CLI and output contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT.

Source: approved feature specification v2, especially FR-SAVE-001–006,
FR-SAVE-025–030, §14.1, and AC-SAVE-016–024.

This document extends the parent
[`cli.md`](../../codemie-cicd-tool/contracts/cli.md). Unchanged `lint`, `apply`,
and `login` behavior remains governed by that contract.

## 1. Command surface

```text
codemie-gitops save --kind Assistant --slug <slug> --file <yaml-path>
                     [--project <project>] [--repo-root <path>] [--url <url>]
                     [--follow-symlinks] [--output text|json]

codemie-gitops save --kind Workflow --slug <slug> --file <yaml-path>
                     [--id <canonical-uuid>]
                     [--project <project>] [--repo-root <path>] [--url <url>]
                     [--follow-symlinks] [--output text|json]

codemie-gitops save --kind Skill --name <name> --file <yaml-path>
                     [--project <project>] [--repo-root <path>] [--url <url>]
                     [--follow-symlinks] [--output text|json]

codemie-gitops save --kind Datasource --repo-name <repo-name>
                     --file <yaml-path>
                     [--project <project>] [--repo-root <path>] [--url <url>]
                     [--follow-symlinks] [--output text|json]
```

Kinds and selector flag names are case-sensitive and exact. Exactly one
kind-applicable selector is accepted. `--id` is Workflow-only and is valid
only with `--slug`; its value is invocation-only and must never enter retained
state or output. `--file` always names a new YAML output. `--output` controls
only the single outcome or diagnostic record and never serializes YAML.

Secret-bearing flags remain absent. An unknown, cross-kind, repeated, or
forbidden selector is `E_USAGE`, exit 2, before network access.

## 2. Local and configuration precedence

Repository root uses the parent `--repo-root` or nearest Git-root behavior.
The effective project is `--project` then repository config `project`. An
absent project or invalid explicit project fails before network and does not
fall back. Target URL remains `--url` > `CODEMIE_URL` > config `url`; token is
environment-only `CODEMIE_TOKEN`.

Before network, the command validates selector syntax and both possible final
paths, including a Skill's derived Markdown path. An existing, symlinked,
escaping, unsafe, or unsupported output path fails locally. The
`--follow-symlinks` flag affects only inherited discovery and existing sidecar
reads; it never permits generated final-path symlinks or parent traversal.

## 3. Invocation ordering

```text
parse CLI
-> resolve repository/config/project and validate output paths
-> create read-only API client and enforce invocation deadline
-> prove visibility where required and resolve exactly one entity
-> read detail/references/content under the save-read manifest
-> reverse-project and reject secret/non-exportable state
-> canonical-render immutable artifact bytes
-> validate the prospective repository through the overlay
-> stage and no-replace publish all artifacts
-> emit one saved outcome
```

The save coordinator exposes only GET-capable adapter interfaces. It does not
receive the apply coordinator's prepared-write dispatcher. Acceptance
instrumentation must prove no POST, PUT, PATCH, or DELETE for every path.

## 4. Success output

JSON output conforms exactly to
[`outcome-v2.schema.json`](outcome-v2.schema.json). Examples:

```json
{"action":"saved","kind":"Skill","project":"demo","name":"triage-skill"}
{"action":"saved","kind":"Workflow","project":"demo","slug":"flow","adoptionRequired":true}
```

Text uses exactly:

```text
saved <Kind> <project>/<natural-key>
```

For an ID-selected unmarked Workflow, the one text line is instead:

```text
saved Workflow <project>/<slug> (adoption required on apply)
```

The text never includes the adoption UUID. JSON provides the machine-readable
`adoptionRequired: true` property. A naturally selected marked Workflow has no
`adoptionRequired` JSON property.

No successful field or text fragment contains a path, URL, server ID, content,
user, time, Git/CI provenance, or adoption UUID.

## 5. Failure output

JSON diagnostics conform exactly to
[`diagnostic-v2.schema.json`](diagnostic-v2.schema.json). Save adds:

| Code | Category | Exit | Fixed meaning |
|---|---|---:|---|
| `E_ENTITY_NOT_FOUND` | reconciliation | 1 | No exact selected entity exists |
| `E_ENTITY_NOT_EXPORTABLE` | reconciliation | 1 | Required authored state cannot be reconstructed safely |
| `E_WORKFLOW_ALREADY_MARKED` | reconciliation | 1 | ID candidate is marked; use natural-key selection |
| `E_OUTPUT_EXISTS` | local-output | 2 | A final target already exists or won a publication race |
| `E_OUTPUT_PATH` | local-output | 2 | Output containment, parent, alias, symlink, or filesystem capability is unsafe |
| `E_OUTPUT_WRITE` | local-output | 2 | Staging or pre-commit publication failed |

Existing `E_AMBIGUOUS_IDENTITY`, `E_IDENTITY_MARKER_INVALID`,
`E_RESOLUTION_UNSTABLE`, `E_MISSING_REFERENCE`, `E_REFERENCE`,
`E_VISIBILITY_UNPROVEN`, `E_API_INCOMPATIBLE`, authentication,
authorization, connectivity, timeout, usage, schema, and internal codes keep
their parent meaning.

Text diagnostics are one fixed template per error code, rendered from typed
enums. Only `E_WORKFLOW_ALREADY_MARKED` includes the fixed guidance “use
natural-key selection”; it does not include an ID or marker value. Output-path
diagnostics omit `source.file`. Server selector values, bodies, exception text,
content, invalid discriminator values, IDs, URLs, tokens, and filesystem paths
are never renderer inputs.

Every failure writes no stdout, exactly one selected-mode diagnostic line to
stderr, no warning, and no YAML/content. There is no debug or panic exception.

## 6. Compatibility

[`outcome-v2.schema.json`](outcome-v2.schema.json) and
[`diagnostic-v2.schema.json`](diagnostic-v2.schema.json) are complete closed
supersets of the v1 schemas. Existing actions and error records remain
byte-compatible. Implementations update the single typed renderer to v2; they
must not dynamically switch schemas by command.
