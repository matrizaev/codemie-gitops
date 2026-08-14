# Documentation map

This directory describes the implementation currently shipped by this repository.
Use this page to choose the correct source when rebuilding or changing the tool.

## Authority order

1. [`../Cargo.lock`](../Cargo.lock) fixes the dependency graph used to build the
   binary.
2. [`../specs/codemie-cicd-tool/contracts/`](../specs/codemie-cicd-tool/contracts/)
   contains closed machine and wire contracts. In particular,
   `declaration-v1alpha1.schema.json` is the accepted authoring language and
   `adapter-manifest-v2.42.0.json` pins the server routes and projections.
   Save adds its versioned contracts under
   [`../specs/save-server-entity/contracts/`](../specs/save-server-entity/contracts/).
3. [`implementation-reference.md`](implementation-reference.md) describes the
   current module boundaries, algorithms, limits, and control flow.
4. [`../ARCHITECTURE.md`](../ARCHITECTURE.md) gives the shorter architectural
   overview.
5. [`../README.md`](../README.md) and [`../CONTRIBUTING.md`](../CONTRIBUTING.md)
   describe operator and contributor workflows.
6. [`yaml-reference.md`](yaml-reference.md) documents every declaration field,
   accepted value, conditional rule, and Datasource variant.

When prose disagrees with a closed JSON Schema or adapter manifest, the closed
contract wins. When prose describes an implementation detail and disagrees with
executable code, update the prose and tests together; do not reinterpret the
contract silently.

## Historical material

Files named `Q-*`, `O-*`, `SEC-*`, and `*-verification*` under
`../specs/codemie-cicd-tool/` are dated review evidence. They intentionally
record failures, superseded designs, and external activation gaps from the time
of review. They are not a cumulative description of the current executable.

The revision history in `../specs/codemie-cicd-tool.md` and superseded ADR
sections explain why the design changed. Rebuild current behavior from the
active contracts and current implementation reference, not from an older
review verdict in isolation.

`../specs/rust-architecture-remediation/` records the completed migration from
repository-wide processing to the current single-file implementation. It is
useful rationale, not a second runtime contract.

## Rebuild baseline

- Rust 1.95, edition 2024.
- Package version: read from `Cargo.toml`.
- Backend contract baseline: CodeMie tag `2.42.0`, commit
  `2a481c290c99bf30ef80aadafa03d876a7f5f732`.
- Build: `cargo build --locked --release`.
- Required checks: `make format`, `make lint`, `make test`.
- Generated declaration DTOs: `build.rs` reads the checked-in declaration JSON
  Schema and writes Rust types into Cargo's `OUT_DIR`; generated source is not
  committed.
