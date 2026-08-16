# Architecture Decision Records

ADRs record *why* one reasonable design was selected over another. They are
rationale for current behavior, not specifications: when an ADR conflicts with
a current executable contract (schema, manifest, code, or test), the executable
contract wins.

An ADR exists only for decisions future maintainers are likely to ask "why?"
about. Routine implementation changes are not ADRs. Superseded decisions are
not preserved in this directory; Git history retains them.

| ADR | Decision |
|---|---|
| [0001](0001-embedded-schema-and-marked-yaml.md) | Embedded closed JSON Schema + marked YAML parsing (schema-first validation) |
| [0002](0002-stateless-single-entity-operation.md) | Stateless, single-entity, single-file, single-binary operation |
| [0003](0003-resolve-and-write-reconciliation.md) | Resolve natural identity, prove authorization, write exactly once, verify |
| [0004](0004-skill-creator-scoped-reconciliation.md) | Skill identity reconciled in the authenticated creator namespace |
| [0005](0005-workflow-identity-and-adoption.md) | Workflow identity via reserved `meta_config` marker and explicit adoption |
| [0006](0006-datasource-ordinary-crud.md) | Datasource ordinary per-kind CRUD with authoritative create-collision |
| [0007](0007-pinned-server-baseline.md) | Pinned backend source baseline and compatibility gate |
| [0008](0008-closed-output-and-diagnostics.md) | Separate success outcomes from allowlist-only failure diagnostics |
| [0009](0009-secret-channels-and-transport-policy.md) | Environment-only secrets, ValidatedUrl, TLS, and redirect policy |
| [0010](0010-save-versioned-reverse-contract.md) | Save reads/reverse projection pinned in a versioned manifest |
| [0011](0011-save-canonical-yaml.md) | Schema-aware canonical YAML serialization |
| [0012](0012-save-stable-snapshot.md) | Bounded observed-stable Skill snapshot |
| [0013](0013-save-direct-single-file-write.md) | Direct create-new single-file save publication |

See [../README.md](../README.md) for the documentation authority model.
