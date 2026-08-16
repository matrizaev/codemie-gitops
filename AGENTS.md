# Repository Instructions

## Development Methodology

This repository follows Specification-Driven Development with a **temporary
artifact lifecycle**. Product behavior is defined by an approved specification
before implementation, and implementation must conform to the approved
specification, architecture, contracts, and task breakdown.

Working artifacts — feature specs, plans, task lists, verification reports,
security-review notes, traceability files, and implementation evidence — are
**temporary**. They are produced while a feature is being designed and built,
and they are deleted (or their unique durable content is distilled) before the
feature is considered complete. Git history is the archive for completed
implementation-process artifacts. Do **not** accumulate completed feature
process files on `main`, and do **not** move them into an `archive/`
directory.

Do not silently resolve conflicts downstream. Update the authoritative artifact
or escalate the conflict to the appropriate role.

## Feature Artifact Lifecycle

A feature follows this lifecycle:

```text
feature intent
    ↓
temporary spec
    ↓
temporary plan
    ↓
temporary tasks
    ↓
implementation
    ↓
tests / contracts / types
    ↓
distill durable knowledge
    ↓
delete temporary artifacts
```

Plans, task lists, verification reports, implementation evidence, and
temporary feature specs are **working artifacts, not permanent project
documentation**. A feature is not complete until its temporary SDD artifacts
have either:

1. been deleted, or
2. had their unique durable content distilled into the appropriate current
   source of truth:
   - behavior → code and tests
   - closed machine interfaces → `contracts/` schemas/manifests
   - architectural rationale → `docs/adr/`
   - implementation reference → `docs/implementation-reference.md`
   - user-facing semantics → `README.md` / `docs/yaml-reference.md`

Durable current knowledge lives in: `ARCHITECTURE.md`,
`docs/implementation-reference.md`, `docs/yaml-reference.md`,
`docs/adr/`, `contracts/`, code, tests, and the top-level user docs. See
[docs/README.md](docs/README.md) for the authority order.

## Temporary Feature Workspace

Prefer keeping active feature artifacts in a Git-ignored working area:

```text
.work/<feature-name>/
├── spec.md
├── plan.md
├── tasks.md
└── optional temporary review notes
```

`.work/` is ignored by Git (see `.gitignore`). If committed feature
specifications are occasionally necessary, they must be removed or distilled
before the merge; do not merge a feature that still carries its process
artifacts in the active tree.

## Roles

For non-trivial development, route work through these roles:

1. `product-spec-owner`
   - Owns product requirements, scope, scenarios, and acceptance criteria.
   - Produces and updates specifications.

2. `solution-architect`
   - Works from an approved specification.
   - Produces architecture plans, ADRs, contracts, data models, migration plans,
     and implementation tasks.

3. `security-reviewer`
   - Reviews architecture before implementation when security-sensitive behavior
     is involved.
   - Reviews implementation again before release when appropriate.

4. `verification-engineer`
   - Performs pre-implementation consistency analysis.
   - Performs post-implementation specification-to-code convergence verification.

5. `implementation-engineer`
   - Implements bounded approved tasks.
   - Must not reinterpret product requirements or architecture silently.

6. `release-engineer`
   - Assesses release readiness.
   - Does not release, publish, tag, merge, or deploy without explicit user
     authorization.

Reviews are lifecycle stages, not permanent files: a security review or
verification report is evidence produced during the stage and does not remain
in the active tree once the feature converges.

## Conflict Ownership

Route conflicts according to ownership:

- Product behavior, scope, acceptance criteria → `product-spec-owner`
- Architecture, data ownership, contracts → `solution-architect`
- Implementation defects → `implementation-engineer`
- Security concerns → `security-reviewer`
- Missing or insufficient evidence → `verification-engineer`
- Deployment and release concerns → `release-engineer`

Do not silently resolve an upstream conflict during a downstream lifecycle stage.

## Repository Scope

The following directories are **reference-only** and are not part of the
product being developed in this repository:

- `codemie/`
- `codemie-ui/`

They may be inspected to understand existing concepts, behavior, architecture,
APIs, UI patterns, terminology, or implementation approaches.

However:

- Do not modify files under `codemie/` or `codemie-ui/`.
- Do not treat either directory as part of the current product architecture.
- Do not include their source files in implementation tasks.
- Do not run migrations, refactors, formatting, dependency updates, or other
  maintenance work against them.
- Do not assume their architecture or implementation choices are requirements
  for the current product.
- Do not copy their implementation blindly. Reuse concepts or approaches only
  when they fit the current product specification and architecture.
- References to these directories in specifications, plans, ADRs, or reviews
  must clearly identify them as external/reference material.

The current product must be designed and implemented outside these
reference-only directories.

## Rust Idioms & Domain Types

- After finishing changes to Rust code, run `make format` and `make lint`
  before reporting the work complete.
- Prefer idiomatic Rust over ad hoc plumbing: small modules, explicit
  ownership, narrow traits, `From`/`TryFrom` conversions, and typed
  request/command/query structs at boundaries.
- Treat Serde request/config structs as boundary DTOs only. Convert them
  immediately into validated command/domain types before enqueueing, running,
  persisting, or applying business rules.
- Do not pass raw `String`, `Vec`, primitive config values, paths, origins,
  image names, or limits deeper than the layer that deserializes or reads them
  when a domain concept exists.
- Model invariants with strong types and validated constructors. Invalid states
  should become unrepresentable once conversion succeeds. Prefer
  `NonZeroUsize`, `NonZeroU64`, small enums, and newtypes over unchecked
  primitives for limits, counts, paths, image names, origins, and addresses.
- Keep validation at conversion boundaries. Prefer
  `TryFrom<RawType>`/`TryFrom<ValidationInput>` or `FromStr` over standalone
  validation functions that return `()` and leave raw values in circulation.
- Express canonical single-input representation conversions with `From`,
  `Into`, `TryFrom`, `TryInto`, or `FromStr`. Keep named mapping helpers when
  conversion needs extra context, workflow policy, or orphan-rule workarounds.

## Errors & Observability

- Use `thiserror` for crate-owned error enums unless there is a concrete reason
  to hand-write `Display`/`Error`.
- Keep errors near the layer that owns the failure and convert at boundaries
  with `From`.
- Return `Result<T, LayerError>` from fallible application, runner, workspace,
  configuration, and API helpers. Do not collapse internal errors into strings
  until crossing an external boundary such as HTTP JSON or logs.
- API errors should preserve typed variants internally and serialize structured
  payloads such as `{ code, error, details }` at the HTTP boundary.
- Propagate errors with `?` when `From`/`#[from]` expresses the layer
  conversion. Use `map_err` or `match` only when adding meaningful context,
  selecting a distinct semantic variant, translating an external boundary, or
  observing/recovering locally.
- Use structured `tracing` logs with explicit fields such as `%job_id`,
  `worker_id`, `status`, and `duration_ms`. Avoid preformatted log strings for
  data that should be queryable.
- Prefer structured tool output over text heuristics. Parse
  `cargo --message-format=json` messages to classify compiler/test outcomes.
