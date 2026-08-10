# Declaration contract: `codemie.epam.com/v1alpha1`

Source: `specs/codemie-cicd-tool.md` v24, FR-001–006,
FR-015, FR-017, FR-021–023, FR-025, FR-027–036, DR-001–012, and
VR-001–016.

Status: NORMATIVE ARCHITECTURE CONTRACT. The exact closed machine contract is
[`declaration-v1alpha1.schema.json`](declaration-v1alpha1.schema.json); the
server projection is pinned by
[`adapter-manifest-v2.42.0.json`](adapter-manifest-v2.42.0.json). Prose in this
document cannot widen either artifact.

Repository defaults and non-secret endpoints use the separate closed
[`repository-config.schema.json`](repository-config.schema.json); that schema
is not part of an entity declaration and cannot widen this authoring envelope.

## 1. Envelope and effective project

Each YAML file contains exactly one closed declaration:

```yaml
apiVersion: codemie.epam.com/v1alpha1
kind: Assistant | Workflow | Datasource | Skill
metadata:
  # project may be omitted only when repository config supplies it
  project: project-name
  # exactly one kind key: slug, repo_name, or name
spec: {}
```

| Kind | Metadata key | Effective identity |
|---|---|---|
| Assistant | `slug` | `(effective project, slug)` |
| Workflow | `slug` | `(effective project, slug)` |
| Datasource | `repo_name` | `(effective project, repo_name)` |
| Skill | `name` | `(effective project, name)` |

`metadata.project` is optional in the syntax schema. Before semantic and
cross-reference validation, the loader computes `effective project` as
`metadata.project` when present, otherwise `.codemie/config.yaml.project`.
Missing or empty in both places is `E_SCHEMA`, exit 2, with no network access.
The effective project participates in identities and payloads but is not
written back to YAML.

The envelope and all nested authoring objects are closed. Unknown keys,
duplicate YAML keys, multiple documents, unsupported tags, runtime/server-ID
positions, lifecycle-control fields, and secret-bearing request members fail
locally. A server UUID is never declaration identity. Workflow adoption UUID is
an invocation-only selector.

## 2. Presence, null, and operation projection

The checked-in schema enumerates the supported authoring subset for all four
entities and every supported Datasource kind. Each admitted field has exactly
one source-pinned class in the adapter manifest:

- authoring-required: present and non-null in YAML; this includes envelope,
  identity, structural/conditional, and any field rejected as null by an
  applicable request;
- optional authorable/null-accepting: omission and explicit YAML null are both
  accepted and normalize to the same typed null;
- authoring-only/transformed: consumed by a bounded transform and not emitted
  under its authoring name;
- operation-inapplicable: absent from that operation without a fabricated
  request member;
- tool-owned or mixed-owned: assembled by its named ownership rule; or
- read-only/prohibited: rejected locally.

For each applicable JSON request, the projector enumerates the manifest's
`optionalNull` set and emits every member explicitly with JSON value `null`
when the author omitted it or wrote YAML null. It performs the same operation
on create and update when the property exists in both request models. It never
looks up, copies, or inserts a concrete server default.

The File Datasource is the pinned non-JSON transport exception: `files` are
multipart parts and scalar model members are query parameters. Its logical
request projection still contains the same typed nulls; because HTTP query
parameters have no JSON-null token, the codec represents such a null by an
absent query member, which FastAPI binds to `None`. The manifest records this
wire representation explicitly. JSON-body adapters always carry literal JSON
null.

Bounded transformations are:

- envelope metadata to server request identity fields;
- natural references to invocation-local server representations;
- Workflow execution author form to `yaml_config` and flattened fields;
- scalar Skill `contentFrom` to inline request `content`;
- source-pinned per-kind Datasource operation fields.

Workflow `meta_config` is not in the optional-null loop. The response carries a
nullable string containing a JSON object. The adapter strictly decodes it,
preserves unmentioned non-reserved members, overlays authored non-reserved
members, adds the reserved identity member last, and sends the canonical
compact JSON string. Malformed/non-object/duplicate-key metadata fails closed.

No interpolation, templating, credential lookup, live-schema expansion, or
default discovery occurs.

## 3. Skill sidecar

Exactly one of the two forms is allowed:

```yaml
spec:
  content: |-
    Inline skill content
```

```yaml
spec:
  contentFrom: ./my-skill.md
```

`contentFrom` is a scalar relative path resolved against the directory of the
declaring YAML file, not the repository root or process directory. It must name
a `.md` regular file which remains inside the canonical repository root. A
symlink is rejected unless the explicit discovery symlink policy permits it;
escapes, cycles, non-files, unreadable files, and bounded-size violations are
`E_SIDECAR`, exit 2. Apply reads the bytes and sends only inline `content`.
Neither the path nor content is eligible for output or diagnostics.

## 4. Workflow identity and authoring

The CLI owns exactly one server-side Workflow identity member:

```json
"codemie.epam.com/gitops/workflow-identity": {
  "version": 1,
  "project": "<effective project>",
  "slug": "<metadata.slug>"
}
```

Authors may supply other `spec.meta_config` members, but the reserved key is
prohibited by the schema. Update projection preserves unmentioned non-reserved
server members and overlays authored non-reserved members. This is identity
metadata, not a generic ownership marker.

Each `spec.execution_config.assistants[]` item has one unique workflow-local
`id` and exactly one form:

- persisted: `assistantRef: {project, slug}`; no inline `system_prompt`,
  `skillRefs`, or `datasourceRefs`;
- inline: non-empty `system_prompt`, no `assistantRef`, and optional inline-only
  `skillRefs: [{project, name}]` and
  `datasourceRefs: [{project, repo_name}]`.

Authors cannot supply `assistant_id`, `skill_ids`, or `datasource_ids` in an
assistant entry. `states[].assistant_id` is a workflow-local graph reference to
`assistants[].id`, not a server asset ID. The adapter maps only persisted
`assistantRef` to server `assistant_id`, and inline `skillRefs`/
`datasourceRefs` to server lists. It retains local actor/state IDs.

## 5. Assistant and Skill natural references

Assistant `context`, `sub_assistants`, and `skills` use only the exact reference
positions admitted by the JSON Schema. The apply adapter validates the exact
server asset and translates it to the source-pinned transport position. Skill
identity uses ADR-007's exhaustive resolver; paths are expanded before its
request is built. Returned IDs exist only in invocation memory.

## 6. Datasource discriminated union

`spec.index_type` selects one closed peer schema. Ordinary authorable kinds in
the pinned baseline are:

| `index_type` | Author form | Update boundary |
|---|---|---|
| `git` | JSON repository fields | pinned Git update fields |
| `svn` | JSON repository fields | pinned SVN update fields |
| `confluence` | flat KB fields | pinned Confluence update fields |
| `jira` | flat KB fields | pinned Jira update fields |
| `xray` | flat KB fields | pinned Xray update fields |
| `azure_devops_wiki` | flat KB fields | pinned Wiki update fields |
| `azure_devops_work_item` | flat KB fields | pinned Work Item update fields |
| `sharepoint` | flat KB fields | pinned SharePoint update fields |
| `file` | metadata plus file/content sidecars | pinned multipart update fields |
| `google` | flat KB fields | pinned Google update fields |

Exact field names, constraints, presence/null class, routes, multipart
handling, and create/update field sets are the JSON Schema plus adapter
manifest. For example, the `google` peer uses authored `googleDoc`,
`setting_id`, and `embedding_model`; fields absent from the pinned update
request are create-only and are not sent on PUT. This technical mapping creates
no distinct entity, permission model, gate, or implementation track.

`provider` is accepted only when an exact deployment-provider schema has first
been reviewed and bundled as a new closed contract. No such schema is present
in this baseline, so v1alpha1 rejects it. The pinned Bedrock vendor-import path
is not ordinary Datasource CRUD and is rejected.

Every integration identifier in a supported kind is an opaque, non-secret
reference to pre-existing external platform configuration. The CLI neither
provisions nor discovers integrations or credentials. Local structure is
validated; CodeMie remains authoritative for existence, type, access, and use.

Ordinary source/content/file/configuration and scheduling body fields are in
scope where the selected operation request exposes them and are transmitted on
every valid apply. `new_project_name`, SharePoint `access_token`, and dedicated
Datasource lifecycle controls are prohibited. File paths are resolved relative
to the declaring YAML, must resolve to repository-contained regular files, and
become byte-preserving `files` multipart parts. `uploaded_files` and
`guardrail_assignments` use compact UTF-8 JSON array strings in their exact
query positions; no path or content may enter diagnostics.

## 7. Cross-reference and evolution rules

Offline lint builds an exact symbol table keyed by
`(apiVersion, kind, effective project, kind key)`. Duplicate declarations make
that symbol ambiguous. Required repository references must exist even when the
referenced entity is applied separately. Workflow-local IDs use a separate
per-declaration table.

A contract change requires an approved specification change when behavior
changes, updates to the JSON Schema and adapter manifest, positive and negative
goldens, and pre-implementation re-verification. A deployment cannot widen the
bundled authoring language.
