# ADR-008: Persist Workflow identity in reserved `meta_config` and adopt explicitly

## Status

Accepted only for metadata codec/preservation and zero-based scanning;
v1 identity/adoption semantics are superseded by ADR-016.

Amended 2026-08-11 for the pinned Workflow pagination origin: both required
enumeration passes are zero-based. The router defaults `page` to `0`, and the
index service queries with `offset(page * per_page)`.

## Context

The current Workflow record has a server UUID and display name but no persisted
slug. CRUD addresses an existing Workflow by server ID. Natural `(project,slug)`
cannot safely select a legacy row by display name. The existing request,
persistence, list, detail, and update paths expose/preserve `meta_config`, which
can carry a Workflow-only identity record without a server migration.

Product specification v24 formally approves the reserved record, exhaustive
resolution, and explicit `--adopt-workflow-id` ceremony. This supersedes
ADR-006's deterministic UUID/new-only approach.

## Decision drivers

- Natural authored/reported Workflow identity
- Safe in-place reconciliation of legacy server-generated-ID Workflows
- No client state and no UUID in YAML/outcomes
- No display-name guessing
- Preservation of non-reserved metadata and history

## Options considered

### A. Exact display-name list resolution

Rejected: display name is mutable and not the approved identity.

### B. Deterministic client UUID

Superseded: useful for new objects but cannot discover/adopt existing UUID rows
sharing the intended slug. Caller-supplied create ID is also only incidental
until contract-tested.

### C. Environment import map or UUID in YAML

Rejected: violates stateless/portable authoring constraints.

### D. Reserved `meta_config` identity plus explicit adoption

Selected and product-approved.

## Decision

### Reserved record

The adapter exclusively owns top-level member:

```json
"codemie.epam.com/gitops/workflow-identity": {
  "version": 1,
  "project": "<exact effective project>",
  "slug": "<exact metadata.slug>"
}
```

It is a closed object. Empty/wrong-typed/unknown/mismatched values are invalid.
Authors may not set this member. Existing non-reserved `meta_config` members are
preserved unless an authored non-reserved value replaces the same member; there
is no unmentioned-member deletion.

The pinned API represents the container as a nullable string containing a JSON
object. Strict decode rejects invalid UTF-8, malformed/duplicate-key JSON,
non-object roots, and invalid reserved values. After merge, canonical encoding
uses compact UTF-8 JSON with recursively sorted object keys, no BOM, and no
non-finite numbers. `meta_config` is mixed-owned and never participates in the
generic omitted-field null loop.

### Normal resolution

Before enumeration, ADR-015 exact membership qualifies creation. ADR-016
restricts ordinary identity to current-principal v2 records. Existing
update/adoption separately requires exact `write`.

Exhaust every relevant Workflow list page across the project and
marketplace-inclusive scopes defined by the source-pinned contract. Each pass
starts at `page=0`: always request page 0 once; for `pages > 0`, request exactly
pages `0..pages-1`; for `pages == 0`, stop after page 0. Each response must echo
the requested zero-based page, use `per_page=100`, and satisfy
`pages=ceil(total/per_page)` with `pages==0` iff `total==0`. Client-filter exact
effective project and reserved record:

- zero -> enumerate unmarked exact display-name candidates as a nonselecting
  guard; any candidate causes `E_ADOPTION_REQUIRED`, otherwise POST create;
- one -> prove project/write ability, read detail for metadata preservation,
  and PUT by returned ID on every valid apply;
- more than one -> `E_AMBIGUOUS_IDENTITY`, exit 1, no write;
- any invalid/conflicting reserved member affecting the project ->
  `E_IDENTITY_MARKER_INVALID`, exit 1, no write;
- missing membership/write authorization -> exit 2.

An invalid page origin, echo, size, or page-count formula is
`E_API_INCOMPATIBLE`, exit 2 before write. Across individually compatible
responses within a pre-write pass, changing pagination fingerprints, repeated
row IDs, accumulated-count mismatch, or scan churn fails closed as entity-
resolution instability, exit 1 with no write. The same compatible instability
detected during post-write full re-resolution is exit 1 with a may-have-
committed result; post-write response-contract or connectivity failure is exit
2 with an uncertain-commit result. Neither post-write classification may be
described as a before-write failure.

Create/update merges the reserved member into the request and a bounded
post-write full re-resolution must find exactly one identity associated with
the expected route ID.
There is no automatic delete, rollback, or blind write retry.

### Explicit legacy adoption

`--adopt-workflow-id <uuid>` is Workflow-only invocation input. Before one PUT,
the CLI requires:

1. canonical UUID syntax;
2. zero valid exact marker matches and no invalid/conflicting marker;
3. by-ID detail in the exact project and provable write capability;
4. candidate contains no valid or invalid reserved identity member;
5. existing metadata is a mergeable object and all non-reserved values can be
   preserved.

Another unmarked row with the same mutable display name neither selects nor
vetoes this explicitly selected candidate.

The PUT persists the marker and reconciles desired state together. Wrong
project, already marked, or unmergeable metadata are exit 1; missing
visibility/write permission is exit 2. Failed post-write
verification reports uncertainty and warns via a stable warning code that the
write may have committed; it does not expose the UUID or values.

### Rename, tampering, and races

Project/slug change is a new identity and never mutates the old marker; the old
Workflow remains because delete is out of scope. Marker removal yields an
unmarked candidate and requires explicit re-adoption. Invalid or duplicate
markers block all relevant writes. CI serialization and governed UI/API writers
are mandatory because the API has no conditional write or unique marker index.

Server UUIDs are ephemeral route selectors only and never author/reported
identity. Workflow-local actor `id` fields are unrelated to this server UUID.

## Consequences

### Positive

- Safely reconciles new and explicitly selected legacy Workflows in place.
- Keeps authoring portable and stateless.
- Preserves history and non-reserved metadata.
- Fails closed on tampering and ambiguity.

### Negative

- Correctness depends on exhaustive privileged reads and metadata preservation.
- The marker is not database-unique and can be changed outside the CLI.
- Adoption needs a one-time privileged UUID input.

### Risks

- Target list projections may omit or truncate `meta_config`.
- UI/API updates could replace unknown metadata.
- Concurrent create/adoption can still create ambiguity.

## Follow-up actions

- Pin and contract-test list/detail/create/update `meta_config` behavior,
  pagination/scopes, project and write indicators, and by-ID authorization.
- Establish serialization, reserved-key governance, inventory, and restore/
  duplicate-remediation runbooks.
- Test empty page 0, one result on page 0, >100 over pages 0 and 1, rejection of
  a first request/response at page 1, invalid markers, marketplace collision,
  adoption checks, merge preservation, rename, drift, and post-write
  uncertainty. Both initial and post-write scans use the same zero-based helper.

## References

- Product specification v31: FR-021/022/028–030/032–034/037, IR-012/013,
  DR-007/008/012, PA-005/006, VR-007–010/016
- Pinned source: `rest_api/routers/workflow.py:109-142` and
  `service/workflow_config/workflow_config_index_service.py:46-163,222-265`
- ADR-006 (Superseded)
- `contracts/http-adapter.md` section 5
