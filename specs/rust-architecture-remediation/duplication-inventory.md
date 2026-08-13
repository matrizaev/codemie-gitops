# Duplication and removal inventory: v33/v3

## Status and decision rule

Consolidate behavior with multiple approved consumers. Delete abstractions whose
only consumers implement superseded repository-walking/closure/staging behavior.

| Cluster | Current locations | Decision | Target owner |
|---|---|---|---|
| Bounded declaration parsing/schema | parse, repository, lint, save overlay | Consolidate around one-file/in-memory declaration input | `input` + `declaration` |
| Repository discovery/view/overlay | discovery, repository, save | Delete from production after caller migration | none |
| Explicit auxiliary path resolution/reads | discovery, parse, datasource adapter | Consolidate narrow declaration-parent containment/no-symlink bounded direct reads; no walking | input |
| File Datasource streaming | datasource adapter/discovery | Retain exact bounded bytes to multipart without temp copies | datasource adapter |
| Natural-reference validation | validate graph + adapters | Split: shape/local IDs in declaration; existence in adapters | declaration/adapters |
| Repository config/root lookup | config, cli | Delete file/root behavior; retain flags/env URL validation | config |
| Cancellation checkpoints | cancellation plus parse/discovery/adapters | Remove from domain APIs; retain runtime-only deadline mechanism if needed | application runtime |
| Save staging/no-replace rename | save publication, tempfile, rustix | Delete; replace with direct create-new writer | filesystem |
| Error translation | layers | Consolidate typed enums/`From`; render only closed diagnostics | owning layer/output |
| Test filesystem fixtures | parse/discovery/save/CLI tests | Keep small `tempfile` test fixtures only if still needed; no production abstraction | each test module |

Dependency removal is evidence-driven. `config`, `tokio-util`, `tempfile`, and
`rustix` may be removed only after all remaining production/test consumers are
audited; no dependency is retained merely to preserve superseded behavior.
