# O-001 production apply controls

This checklist activates the race-prevention controls required for Workflow,
Skill, and Datasource identity writes. The repository supplies configuration
and reproducible local checks; it does not activate GitHub or GitLab settings,
create protected environments, freeze platform writers, or perform a remote
mutex drill.

Do not commit completed activation evidence or inventory snapshots. Store them
in the adopting organization's change-control system. Copy
`activation-evidence.example.json` there and retain only non-sensitive run or
pipeline references. If a short-lived local copy is required, use only
`ops/o001/activation-evidence.local.json` or `ops/o001/inventory.local.json`;
those exact names are ignored. Remove them after external evidence capture.

## Named ownership

| Control | Accountable role |
|---|---|
| Per-environment serialization and CI activation | Release Engineering |
| UI/API identity-writer governance and emergency changes | CodeMie Platform Administrator |
| Complete-visibility marker and duplicate inventory | CodeMie Platform Identity Administrator |
| Concurrent-writer or uncertain-write incident | CodeMie Platform Operations Incident Commander |

These roles are normative for this repository's control boundary. An adopting
organization may map them to named teams or people in its change-control
system, but must not leave a control unowned.

## Checked-in gate

Run:

```text
make o001-check
```

A pass proves the checked-in GitHub/GitLab mutex keys, exact-artifact promotion,
role policy, inventory analyzer, fixtures, and non-destructive remediation
rules converge. The result reports `activationStatus: required`; it is not
evidence that either provider or a CodeMie environment has been changed.

## Identity inventory gate

The CodeMie Platform Identity Administrator must use a principal with complete
project visibility and platform-owned administrative export tooling. Normalize
the ephemeral export to this shape:

```json
{
  "workflows": [
    {
      "markerPresent": true,
      "marker": {"version": 1, "project": "project-a", "slug": "release-flow"}
    },
    {"markerPresent": false}
  ],
  "skills": [{"project": "project-a", "name": "release-skill"}],
  "datasources": [{"project": "project-a", "repo_name": "docs_repo"}]
}
```

Do not include server IDs, creators, payloads, credentials, URLs, or response
bodies. Pipe the temporary snapshot into:

```text
python3 scripts/o001_inventory.py < ops/o001/inventory.local.json
```

The gate passes only with:

- zero invalid Workflow reserved markers;
- zero duplicate exact Workflow `(project, slug)` markers;
- zero invalid Skill identity records;
- zero duplicate exact Skill `(project, name)` keys;
- zero invalid Datasource identity records;
- zero duplicate exact Datasource `(project, repo_name)` keys.

Unmarked legacy Workflows are counted for explicit review but are not silently
selected or modified. Record their disposition in the inventory evidence.
Remove the temporary snapshot after the change-control record captures the
pass/fail result.

## Exact-artifact and credential boundary

The GitHub `prepare` job and GitLab `codemie_gitops_build` job run without a
protected deployment environment. They execute preflight, lint, tests, a locked
release build, a release-binary smoke check, and SHA-256 generation before
publishing the artifact. Build runners must have Rust, Python 3, Make, and the
literal `x86_64-unknown-linux-musl` target plus `musl-gcc`. GitLab selects this
preconfigured runner with the checked-in `codemie-gitops-musl-build` tag and
fails its explicit toolchain precondition instead of promoting a host-dependent
binary.

The protected apply jobs consume only that job artifact. They verify the
checked-in SHA-256 file, restore executable permission, and execute the binary;
they do not run Cargo or repository Python. GitHub references `CODEMIE_URL` and
`CODEMIE_TOKEN` only on the final binary step. GitLab exposes those two values
only as protected variables scoped exactly to the static `production`
environment; its protected runner must provide the binary's runtime and trusted
core checksum/file utilities.

## Provider activation and mutex drill

For every canonical lowercase target environment:

1. Release Engineering creates each selected protected GitHub environment and
   the static GitLab `production` and `staging` drill environments, restricts
   deployment branches, and configures required approval.
2. Release Engineering configures GitHub environment secrets and GitLab
   protected/masked `CODEMIE_URL` and `CODEMIE_TOKEN` variables scoped exactly
   to `production`. It protects the default deployment ref, assigns the
   `codemie-gitops-production-protected` runner only to protected refs, and
   provisions the separate `codemie-gitops-musl-build` runner with the literal
   musl target/toolchain. It restricts pipeline-variable overrides to the
   approved deployment role.
   The checked-in workflows never acquire or print a token.
3. The CodeMie Platform Administrator approves a complete-visibility inventory
   and freezes Workflow, Skill, and Datasource identity writers in the UI,
   scripts, and other API clients.
4. For GitLab, set `CODEMIE_GITOPS_APPLY=true` only on a protected default-ref
   pipeline and supply the three approved preflight values. Dispatch two static
   `codemie_gitops_production_mutex_drill` jobs close together. For GitHub,
   dispatch two `mutex-drill` runs for the same protected environment. Confirm
   their 30-second controlled sections do not overlap.
5. Dispatch a GitHub drill for a different canonical environment and the
   static GitLab `codemie_gitops_staging_mutex_drill`. Confirm each different
   environment key is independent. Cross-environment overlap is permitted.
6. During the first separately authorized production deployment window,
   confirm the promoted artifact checksum passes before binary execution. The
   promotion run/pipeline reference must differ from all mutex-drill references;
   the mutex drill itself remains non-writing.
7. Record protected-environment settings, required approvals, deployment-branch
   and protected-ref restrictions, protected runner and variable-override
   controls, exact production secret scope, pairwise-distinct same/different
   environment and artifact-promotion references, non-overlap, checksum,
   inventory, explicit clean/frozen assertions, different-environment
   independence, distinct freeze/inventory evidence, verifier role, and a UTC
   verification time using the activation-evidence template.
8. Validate the completed external evidence copy:

   ```text
   python3 scripts/check_o001_controls.py --activation-evidence /secure/evidence.json
   ```

GitHub uses `codemie-gitops-apply-<environment>` with queued pending runs.
GitLab uses the static `codemie-gitops-apply-production` and
`codemie-gitops-apply-staging` resource groups; no pipeline variable can alias
one environment to another. The provider-side drill remains mandatory because
local checks cannot prove remote settings or artifact transfer.

## Deployment window

Before selecting `apply`, Release Engineering confirms:

- the checked-in gate passes and provider protection/mutex evidence from steps
  1–5 is approved; the full activation-evidence gate intentionally cannot pass
  until the first authorized apply supplies step 6's checksum evidence;
- the complete-visibility inventory is clean and unmarked Workflows are
  reviewed;
- the external identity-writer freeze is active;
- the non-sensitive control-evidence reference identifies those approvals;
- no earlier uncertain-write or concurrent-writer incident remains open.

The first apply remains gated by the workflow preflight booleans, protected
environment approval, protected ref/runner controls, and external change
approval. After its checksum-confirmed execution, Release Engineering records
the distinct artifact-promotion reference and completes steps 7–8; only then
does the full activation-evidence check report `activation-pass`.

Only the serialized CI principal writes during the window. The CodeMie Platform
Administrator may use an emergency writer only after Release Engineering stops
the queue and the Incident Commander opens remediation.

## Concurrent-writer remediation

1. Release Engineering stops new applies for the affected environment without
   canceling an in-flight modifying request.
2. The CodeMie Platform Administrator closes the bypassing UI/API writer and
   preserves platform audit evidence outside this repository.
3. The Identity Administrator reruns the complete Workflow marker, Skill, and
   Datasource inventory.
4. The Incident Commander assigns manual platform remediation for every
   invalid marker or duplicate. The CLI must not choose, delete, roll back, or
   retry an ambiguous entity.
5. The Identity Administrator proves the inventory clean. Release Engineering
   resumes applies only after the Platform Administrator re-approves the
   writer freeze.

## Uncertain-write remediation

An `E_WRITE_UNCERTAIN` result or a verification-time connectivity/
compatibility failure means the write may have committed.

1. Do not rerun apply and do not infer success from the failed CI job.
2. Release Engineering holds the environment queue and the Platform
   Administrator freezes all identity writers.
3. The Identity Administrator re-enumerates the affected natural key with
   complete visibility and runs the full inventory.
4. If identity is invalid or duplicated, the Incident Commander directs manual
   platform remediation. No automatic retry, deletion, rollback, or tie-break
   is permitted.
5. After a clean inventory and explicit approvals, Release Engineering may
   start one new serialized apply from the desired Git state.

Broader token-acquisition examples, general CI usage, release recovery, and
rollback documentation remain O-002 scope.
