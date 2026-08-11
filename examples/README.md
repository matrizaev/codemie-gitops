# Portable examples

This directory is inert reference material. It is not a provider-active root
workflow and no command runs merely by checking out the repository.

`repository/` is a complete sample repository containing one declaration for
Assistant, Workflow, Skill, and Datasource. The Datasource uses the opaque,
non-secret placeholder integration reference `replace-with-project-setting`;
its presence demonstrates offline authoring only and does not authorize live
execution. Filenames and directories are conventions for reviewability, not
entity identity. Identity comes only from the declaration kind, effective
project, and its `slug`, `name`, or `repo_name` metadata.

From `examples/repository`, validate in dependency order without credentials:

```sh
codemie-gitops lint --file skills/example-skill.yaml --repo-root .
codemie-gitops lint --file datasources/example-datasource.yaml --repo-root .
codemie-gitops lint --file assistants/example-assistant.yaml --repo-root .
codemie-gitops lint --file workflows/example-workflow.yaml --repo-root .
```

Only after target authorization, protected endpoint injection, and the O-001
serialization/inventory controls are active may an operator login once and
apply individual files in the same order. Repeat apply always performs an
update. Never substitute local-auth mode in CI.

`ci/github-actions.yml` and `ci/gitlab-ci.yml` are portable samples to copy and
adapt. Provider-side protected environments, reviewers, protected refs/runners,
masked environment-scoped variables, and variable-override restrictions must
be configured remotely; checked-in YAML cannot prove activation. Both samples
promote a checksummed artifact built without deployment credentials into the
protected job without rebuilding.

Token delivery is provider-specific. GitHub captures one fresh `login` token,
immediately registers it with native `add-mask`, and reuses it only inside that
protected step. GitLab has no stable runtime add-mask command, so its protected
job consumes a pre-supplied environment-scoped protected+masked
`CODEMIE_TOKEN`, does not invoke `login`, and reuses the token only in that job
process. Neither path persists, transfers, echoes, or simulates masking.

Recovery entry points:

- [Git revert followed by a new apply](../ops/o002/GIT_REVERT_RECOVERY.md)
- [exact-ID Workflow adoption](../ops/o002/WORKFLOW_ADOPTION.md)
- [uncertain-write hold and inventory](../ops/o002/UNCERTAIN_WRITE.md)
