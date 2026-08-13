# Portable examples

This directory is inert reference material. It is not a provider-active root
workflow and no command runs merely by checking out the repository.

`repository/` contains independent samples, one declaration for
Assistant, Workflow, Skill, and Datasource. The Datasource uses the opaque,
non-secret placeholder integration reference `replace-with-project-setting`;
its presence demonstrates offline authoring only and does not authorize live
execution. Filenames and directories are conventions for reviewability, not
entity identity. Identity comes only from the declaration kind, effective
project, and its `slug`, `name`, or `repo_name` metadata.

From `examples/repository`, validate each selected file without credentials:

```sh
codemie-gitops lint --file skills/example-skill.yaml
codemie-gitops lint --file datasources/example-datasource.yaml
codemie-gitops lint --file assistants/example-assistant.yaml
codemie-gitops lint --file workflows/example-workflow.yaml
```

Only after target authorization and protected endpoint injection may an
operator login once and apply individual files in the same order. Repeat apply
always performs an update. Never substitute local-auth mode in CI.

`ci/codemie-gitops-apply.yml` and
`ci/codemie-gitops-apply.gitlab-ci.yml` are portable samples to copy and
adapt. Provider-side
protected environments, reviewers, protected refs/runners,
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
