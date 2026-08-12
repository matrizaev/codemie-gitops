# V-000 non-mutating target qualification

`scripts/v000_target.py` is a local harness for the pinned CodeMie 2.42.0
consumed GET contract. It cannot construct a modifying request. Local tests do
not complete V-000; a separately authorized V-000B execution against a named
target must supply the target-specific evidence.

The checked-in `enterprise-smoke.example.json` is deliberately incomplete and
must fail closed. Copy it to an ignored `ops/v000/<run>.local.json`, fill it
with separately authorized non-production values, and place run-scoped
Assistant/Workflow/Skill declarations in an ignored
`ops/v000/run-<id>.local/` directory. Datasource cannot appear anywhere in the
smoke manifest and has no exception or selector. Its compatibility is probed
only through bounded GET requests.

Manifest declaration paths are relative to that isolated declaration root,
not the product repository. The harness rejects escapes and every symlink
component, binds each parsed kind/effective project/natural key to the manifest,
and invokes offline `lint` through the exact staged binary with only that
directory as `--repo-root`. Unrelated YAML elsewhere is never discovered.

Before any authentication or network access the harness:

1. strictly validates the closed manifest as exactly one Assistant, Workflow,
   and Skill;
2. verifies the staged regular non-symlink binary against the operator-supplied
   fixed SHA-256; and
3. when requested, parses only the workspace-root `.env` as non-evaluated data
   after ignored/untracked/history, ownership, `0600`, and no-symlink checks.

The loader accepts only documented `CODEMIE_*` inputs and
`CODEMIE_TEST_PROJECT`. It never uses `source`, `.`, `eval`, interpolation, or
command substitution. CI must not use this dotenv path.

Example invocation (placeholders are intentionally non-operational):

```sh
set +x
install -D -m 0555 target/release/codemie-gitops ./staged/codemie-gitops
sha256sum ./staged/codemie-gitops
python3 scripts/v000_target.py --preflight \
  --binary ./staged/codemie-gitops \
  --sha256 '<fixed-64-lowercase-hex-digest>' \
  --url 'https://authorized-target.example.invalid/' \
  --project '<authorized-project>' \
  --smoke-manifest ops/v000/<run>.local.json \
  --declaration-root ops/v000/run-<id>.local \
  --evidence ops/v000/<evidence>.local.json
```

The staged copy must be owner/group/world non-writable (`0555`), executable,
regular, and non-symlink. Supply the fixed digest printed for that exact copy;
do not hash a different build path. V-000A requires Linux `memfd`, file seals,
and `/proc/self/fd`. It opens the authored path component-by-component without
following symlinks or consulting `PATH`, copies and hashes that one open file
into a sealed in-memory executable, and invokes only those immutable bytes.
Both `codemie-gitops` and `./staged/codemie-gitops` are interpreted relative to
the harness invocation directory, never relative to the later lint directory.

Use `--dotenv .env` only for a reviewed local execution. An already supplied
`CODEMIE_TOKEN` is consumed in memory; otherwise the exact staged binary is
invoked once as `codemie-gitops login`. Credential POST logic is never
reimplemented and login stdout is never persisted. Client secret/password are
removed from the harness environment after token acquisition.

The read transport uses verified HTTPS, exact-origin bearer attachment, no
proxy or redirect following, a 60-second request timeout, 300-second invocation
deadline, 8 MiB body cap, 1,000-page/100,000-item caps, strict consumed-member
decoding, and additive-only tolerance. Skill compatibility and collision
evidence are deliberately separate: a general `project_with_marketplace` scan
without `search` must observe a non-empty page 0, while a second exhaustive
pinned search-hint scan client-filters exact `(project,name)` and requires zero
matches. Workflow exhausts both project-visible and marketplace scopes and
requires the exact reserved `(project,slug)` marker to be absent. A malformed
target-project marker or an unmarked target-project row whose display name
equals the concrete Workflow's authored `spec.name` is also a collision in
either scope. Assistant requires its exact slug/project lookup to return not
found. All three absence checks complete before an in-memory qualification
proof is created. No natural key or display-name value is persisted.

Evidence contains only the fixed schema
and version identifiers, pass/fail status, the actual validated staged-binary
SHA-256, fixed actor/project/role/writer-window binding categories, safe local
request IDs, fixed Assistant/Workflow/Skill absence categories, and bounded
page-0 counts. It excludes session identifiers, target URLs, actors,
credentials, natural keys, bodies, entity values, and raw exceptions.

Qualification is read-only and is not write authorization. The harness creates
an in-memory qualification proof bound by object identity to the open sealed
binary capability and bearer session; it is never serialized and cannot be
reconstructed from the evidence file. A future V-003 integration must consume
that proof before it can claim same-execution/session/digest binding, rerun the
required checks, and perform its separately required digest check before every
apply. No live request or write is part of the local V-000A task.

Version identifiers use separate namespaces: `schemaVersion: 1` is the
sanitized evidence-envelope schema, while `adapterManifestVersion: 3` binds the
record to the consumed adapter contract. Legacy evidence containing
`manifestVersion: 2`, omitting `adapterManifestVersion`, or naming another
adapter version is stale and cannot qualify V-000B or V-003.

The smoke authorization `actor` is the authenticated `/v1/user.user_id`, not
the user's email. Email is used only where an authentication mode requires it.
