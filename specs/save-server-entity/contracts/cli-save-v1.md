# Save CLI and output contract v1 (revised for specification v3.4)

Status: NORMATIVE ARCHITECTURE CONTRACT.

## 1. Command surface

```text
codemie-gitops save --kind Assistant --slug <slug> --file <yaml-path>
                     --project <project> [--url <url>] [--output text|json]
codemie-gitops save --kind Workflow --slug <slug> --file <yaml-path>
                     --project <project> [--id <uuid>]
                     [--url <url>] [--output text|json]
codemie-gitops save --kind Skill --name <name> --file <yaml-path>
                     --project <project> [--url <url>] [--output text|json]
codemie-gitops save --kind Datasource --repo-name <repo-name>
                     --file <yaml-path> --project <project>
                     [--url <url>] [--output text|json]
```

`--repo-root`, `--follow-symlinks`, force/replace, and secret-bearing flags are
unknown and fail `E_USAGE` before network access.

## 2. Configuration and ordering

Project is the explicit `--project`. Target URL is `--url` then `CODEMIE_URL`;
token is environment-only. No local config/root discovery occurs.

Ordering is:

```text
parse/validate command
-> validate output form, parent, and detectable absence
-> bounded read-only compatibility and entity resolution
-> strict reverse projection and confidentiality checks
-> canonical one-file YAML rendering
-> validate only generated declaration in memory
-> direct create-new/write requested final path
-> emit saved only after complete write
```

## 3. Output

Text success is exactly `saved <Kind> <project>/<natural-key>`. An ID-selected
unmarked Workflow appends ` (adoption required on apply)`. JSON success
conforms to `outcome-v2.schema.json`: it contains `action: "saved"`, `kind`,
`project`, and exactly the applicable natural-selector field; the ID-selected
unmarked Workflow additionally contains `adoptionRequired: true`. Success
contains no URL, path, server ID, content, time, user, adoption UUID, or
external provenance. Failures leave stdout empty and emit exactly one safe
diagnostic.

`E_OUTPUT_EXISTS` is exit 2 for an existing target/pre-create race.
`E_OUTPUT_PATH` is exit 2 for an unsafe/invalid parent or target.
`E_OUTPUT_WRITE` is exit 2 for any failure after direct final creation; an
incomplete final path may remain and is not removed. No such failure emits
`saved`.

## 4. Compatibility and server safety

The pinned compatibility and bounded-response checks precede final creation.
All server requests are read-only. GET retries follow the existing transport
policy; no modifying request is possible.
