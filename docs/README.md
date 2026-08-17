# Documentation map

This page is the authority model for the repository's knowledge surface. Use it
to choose the correct source when rebuilding or changing the tool.

## Authority order

1. **Executable contracts/schemas** — [`../contracts/`](../contracts/)
   contains the closed machine and wire contracts:
   `declaration-v1alpha1.schema.json` is the accepted authoring language,
   `adapter-manifest-v2.42.0.json` pins server routes and projections,
   `openapi.json` pins the backend wire shapes, and the
   `*-v1.md`/`*.schema.json` files pin the CLI, HTTP adapter, canonical
   YAML, save publication, and reverse-projection contracts.
2. **Executable code and tests** — `src/` and `tests/` represent the
   current implementation behavior. When prose disagrees with code, update the
   prose and tests together; do not reinterpret a contract silently.
3. **Current implementation reference** —
   [`implementation-reference.md`](implementation-reference.md) describes
   module boundaries, algorithms, limits, and control flow.
4. **Architecture documentation** — [`../ARCHITECTURE.md`](../ARCHITECTURE.md)
   gives the shorter architectural overview: boundaries, flows, invariants,
   principles, non-goals, and responsibility ownership.
5. **User/operator documentation** — [`../README.md`](../README.md) and
   [`../CONTRIBUTING.md`](../CONTRIBUTING.md) describe operator and
   contributor workflows; [`yaml-reference.md`](yaml-reference.md) documents
   the declaration language for authors.
6. **ADRs for rationale** — [`adr/`](adr/) records *why* one reasonable
   design was selected over another.

## Precedence rules

- Code and tests represent current implementation behavior.
- Executable contracts (schemas, manifests, pinned OpenAPI) **override
  conflicting prose** for closed machine interfaces.
- ADRs explain rationale but do **not** override current executable contracts.
- If a concrete value or rule is already authoritative in code, a schema, a
  generated contract, configuration, or a test, prefer linking to or describing
  the ownership of that fact rather than duplicating the exact value in prose.

## Historical material

Completed implementation-process artifacts (feature specs, plans, task lists,
verification reports, security-review notes, traceability files, and migration
inventories) are **intentionally absent** from the active repository tree. Git
history is their archive. Do not reconstruct current behavior from older
review verdicts or superseded design clauses.

## Rebuild baseline

- Rust 1.95, edition 2024.
- Package version: read from `Cargo.toml`.
- Backend contract baseline: CodeMie tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732` (see
  [`../contracts/source-baseline.md`](../contracts/source-baseline.md)).
- Build: `cargo build --locked --release`.
- Required checks: `make format`, `make lint`, `make test`.
- Generated declaration DTOs: `build.rs` reads the checked-in declaration JSON
  Schema at `contracts/declaration-v1alpha1.schema.json` and writes Rust
  types into Cargo's `OUT_DIR`; generated source is not committed.
