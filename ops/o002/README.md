# O-002 operator index

These local controls do not prove remote provider activation. Before a
production apply, complete O-001 provider protection, per-target serialization,
complete-visibility identity inventory, writer freeze, and exact-artifact
evidence. CI credentials must be protected, masked, environment-scoped values;
fork and pull-request work remains secret-free.

Use the exact checksummed binary built and tested outside the credential
boundary. GitHub performs one fresh login in the protected apply step and its
immediately following command invokes native `::add-mask::` before any other
command or output; the token remains only in that step's memory. GitLab never
invokes login: its protected job consumes only a pre-supplied
environment-scoped, protected, masked `CODEMIE_TOKEN` in process memory. Both
providers apply one declaration at a time. Apply always writes, including
repeat apply; neither provider persists, transfers, re-emits, or simulates
masking for a token.

Recovery is forward-only and operator-controlled:

- [Git revert/new apply](GIT_REVERT_RECOVERY.md)
- [Workflow exact-ID adoption](WORKFLOW_ADOPTION.md)
- [uncertain-write hold and inventory](UNCERTAIN_WRITE.md)

Never use a recovery procedure as permission for a live operation. Datasource
is excluded from the enterprise V-003 smoke; its portable declaration is an
offline example only.
