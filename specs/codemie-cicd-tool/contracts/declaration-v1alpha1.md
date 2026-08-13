# Declaration contract: `codemie.epam.com/v1alpha1`

Source: `specs/codemie-cicd-tool.md` v33.

Status: NORMATIVE ARCHITECTURE CONTRACT. The exact closed machine contract is
`declaration-v1alpha1.schema.json`; server projection is pinned by
`adapter-manifest-v2.42.0.json`.

## 1. Envelope

Each selected YAML contains exactly one closed declaration:

```yaml
apiVersion: codemie.epam.com/v1alpha1
kind: Assistant | Workflow | Datasource | Skill
metadata:
  project: project-name
  # exactly one kind identity: slug, repo_name, or name
spec: {}
```

Project is explicit. There is no repository-default materialization. Unknown
keys, duplicate keys, multiple documents, unsupported tags, server IDs,
lifecycle controls, and secret-bearing request members fail locally.

## 2. Presence and projection

The adapter manifest classifies every admitted field as required,
optional-null, transformed, operation-inapplicable, tool/mixed-owned, or
read-only/prohibited. Applicable omitted/null optional fields become JSON null;
no server default is discovered or fabricated. Existing Workflow meta-config,
File Datasource transport, and kind-specific projection rules remain normative.

## 3. Skill content

Skill requires exactly one of inline content or an explicit sidecar:

```yaml
spec:
  content: |-
    Inline skill content
```

```yaml
spec:
  contentFrom: ./my-skill.md
```

`contentFrom` is resolved relative to the selected declaration's parent. It is
one explicitly named, bounded `.md` regular-file read under the command timeout.
Absolute/escaping/dot/symlinked/non-regular/unreadable targets fail locally;
there is no repository root, directory enumeration, or symlink-following mode.
The content is validated against the Skill content bounds and projected inline
to the server. Path/content never enter diagnostics.

## 4. References

Natural references carry explicit `{project, natural-key}` structure in the
positions admitted by the schema. Offline lint validates only structure and
Workflow-local actor/state relationships. It does not require or inspect a
neighboring declaration. Apply resolves every natural reference against the
server through the source-pinned kind adapter and keeps returned IDs only in
invocation memory.

## 5. Entity-specific rules

- Workflow identity remains the reserved server `meta_config` record; explicit
  adoption is invocation-only.
- Assistant and Skill references use their exact typed natural keys.
- Datasource remains the closed discriminated union in the JSON Schema and
  adapter manifest; external integration values are opaque pre-existing server
  references.
- File Datasource `spec.files` contains 1–10 distinct explicit relative path
  strings. Each resolves from the selected declaration's parent and is read
  directly under the 32 MiB per-file, 128 MiB aggregate, and command-timeout
  bounds. Absolute/escaping/symlinked/non-regular/unreadable/duplicate targets
  fail locally. Apply uses the safe basename and exact bytes for repeated
  multipart `files` parts without a temporary/staging copy. `uploaded_files`
  remains a retained server-filename list, not a local path source.
- No entity uses a server UUID as authored identity.

## 6. Evolution

Behavior changes require an approved specification revision, schema/manifest
updates, positive and negative goldens, and pre-implementation verification.
No deployment or local file layout can widen the authoring language.
