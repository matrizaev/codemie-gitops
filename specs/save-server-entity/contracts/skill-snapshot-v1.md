# Skill read snapshot contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT.

Source: approved feature specification v2, FR-SAVE-011/019/020/030,
DR-SAVE-006, IR-SAVE-002/006, QR-SAVE-005/006, and AC-SAVE-006–009.

## 1. Resolution

The adapter first proves complete exact-project visibility through
`GET /v1/user`, then exhausts zero-based pages of:

```text
GET /v1/skills?filters={project,scope:project_with_marketplace,search}
               &page={page}&per_page=100
```

Exact client filtering uses project and name only. Creator, list order, newest
row, and ID never break a tie. Exactly one row supplies an invocation-local ID.

## 2. Observation sequence

For that ID, sequentially perform:

```text
detail A  GET /v1/skills/{id}
payload A GET /v1/skills/{id}/companion-files/content?path={path} for every path
detail B  GET /v1/skills/{id}
payload B GET /v1/skills/{id}/companion-files/content?path={path} for every path
detail C  GET /v1/skills/{id}
```

The `path` query value is percent-encoded. Payload order is normalized-path
Unicode scalar order. Calls are sequential, use the inherited bounded GET
retry policy, and share the 300-second invocation deadline.

The separate companion metadata list route is not needed because detail
already contains the same metadata. It must not be added as a third metadata
authority without a manifest revision.

## 3. Stable detail fingerprint

Details A/B/C strictly decode and compare:

- `id`, `name`, `project`, and `updatedDate`;
- `description`, main `content`, `visibility`, and `categories`;
- closed safe projections of `toolkits` and `mcp_servers`;
- `companion_files` metadata;
- `enabled_builtin_subagents`; and
- every field classified as required by the reverse manifest.

Ordering is normalized only where the manifest says set semantics. Main
content and ordered domain arrays compare byte/value exactly. Server/audit
fields other than `updatedDate` are decoded only as classified ignored fields
and do not enter the fingerprint.

All three fingerprints must be identical. A stable-shape difference is
`E_RESOLUTION_UNSTABLE`, exit 1. A missing field, wrong type, duplicate unknown
field, or unknown contracted object member is `E_API_INCOMPATIBLE`, exit 2.

## 4. Companion integrity

For each metadata entry:

- normalize the relative path with the pinned server rules;
- reject absolute, escaping, empty, NUL/control, reserved `SKILL.md`, or
  duplicate paths;
- require response path to equal the requested normalized path;
- require exact MIME type and encoding from metadata;
- decode `text` as UTF-8 bytes and `base64` with strict canonical base64;
- require decoded byte length to equal `size_bytes`; and
- require A and B decoded bytes and metadata to be identical.

The declaration stores the server response's content string together with its
validated encoding and decoded `size_bytes`, as required by the existing
schema. It does not create companion sidecars.

## 5. Main sidecar

Main `content` from the stable detail becomes exact UTF-8 bytes in the derived
Markdown sidecar. No BOM, newline conversion, or trailing newline is added.
The YAML contains only the sidecar basename in `spec.contentFrom` and omits
`spec.content`.

## 6. Confidentiality and retention

IDs, `created_by`, abilities, counts, dates other than the transient snapshot
marker, raw bodies, paths in diagnostics, and nested credential values never
enter artifacts or output. Nested integration selections retain only the
schema-approved opaque `id` and optional `alias`; credential values and MCP
config/auth payloads are non-retaining ignored fields.

All details and payloads are transient memory and are dropped after immutable
artifact bytes are built or on any failure. There is no response cache.
