# Canonical declaration YAML contract v1

Status: NORMATIVE ARCHITECTURE CONTRACT.

Source: approved feature specification v2, DR-SAVE-001/003/006/007,
FR-SAVE-013/014/024, and QR-SAVE-001/007/009.

## 1. Input domain

The emitter accepts only a reverse-projected JSON-compatible AST that already
validates against the checked-in `codemie.epam.com/v1alpha1` schema. Mapping
keys are strings. Values are null, Boolean, finite JSON number, string, array,
or mapping. Tags, anchors, aliases, merge keys, non-string keys, NaN, and
infinities have no representation and are rejected before emission.

The emitter is a product component, not a general YAML pretty-printer. A
library may supply escaping primitives, but library default ordering, quoting,
or line wrapping is not normative.

## 2. Document and indentation

- UTF-8 without BOM.
- LF line endings regardless of host platform.
- No `%YAML` directive and no `---` or `...` marker.
- Two spaces per mapping or sequence nesting level; no tabs for indentation.
- No trailing spaces.
- Exactly one LF follows the document's last non-chomping output byte.
- Mapping form is `key: value`; sequence form is `- value` with nested values
  on following indented lines.
- Empty mapping and sequence values are `{}` and `[]`.

## 3. Property order

The document order is `apiVersion`, `kind`, `metadata`, `spec`.
`metadata` order is `project` then `slug`, `name`, or `repo_name` as applicable.

Every closed object uses the property order in
`declaration-v1alpha1.schema.json` as written in that file. The implementation
maintains a generated-or-verified order table keyed by schema definition and
fails its contract test if a closed schema property lacks an order entry.

Free-form object keys, including Workflow `meta_config`, `custom_metadata`,
tool arguments, and allowed free-form nested maps, are sorted by Unicode scalar
value sequence. Comparison is locale-independent and does not use UTF-16 code
unit order. Known properties precede free-form children only where the schema
explicitly defines both.

## 4. Arrays

Domain-ordered arrays preserve the normalized server order. This includes
Workflow actors/states/tools/custom nodes, Assistant context/sub-assistants,
and conversation starters.

Skill companion files are set-semantic for canonical output: normalize and
validate each path, reject duplicates, then sort by normalized `path` using
Unicode scalar order. Other manifest-classified set-semantic arrays use their
manifest ordering and reject duplicates. No array is sorted merely because its
items are scalar.

## 5. Scalar spelling

- Null: `null`.
- Boolean: `true` or `false`.
- Integer: base-10, no leading `+`, no leading zero except `0`.
- Negative zero integer: `0`.
- Finite non-integer numbers: the shortest round-tripping decimal spelling
  selected by the Rust JSON number implementation, lowercase `e`, no `+` in
  the exponent, and exponent leading zeros removed. Negative floating zero is
  `-0.0`. A number whose lexical form is not uniquely covered by these rules
  is rejected until this contract is versioned.
- Empty string: `""`.

A single-line string is emitted in plain style only when all are true:

1. it matches `^[A-Za-z_][A-Za-z0-9_./-]*$`;
2. its lowercase form is not `null`, `true`, `false`, `yes`, `no`, `on`,
   `off`, `nan`, or `inf`;
3. it is not parseable as a JSON number; and
4. it has no YAML timestamp-like prefix `YYYY-`.

All other single-line strings use double-quoted style. Double-quoted escaping
is JSON escaping with `\"`, `\\`, `\b`, `\f`, `\n`, `\r`, and `\t`; C0/C1
controls and U+2028/U+2029 use lowercase `\u` escapes. Other Unicode scalar
values are emitted as UTF-8. Lone surrogates cannot enter the input domain.

Mapping keys use the same plain-or-double-quoted decision.

## 6. Multiline strings

A string containing LF and no CR or disallowed control character uses a
literal block scalar. The emitter never folds text and never wraps lines.

- Zero trailing LFs: `|-`.
- Exactly one trailing LF: `|`.
- Two or more trailing LFs: `|+`.

Every content line is indented two spaces beyond its key/sequence position.
Leading and trailing spaces inside content lines are preserved. Empty content
lines are emitted explicitly. The `|+` form includes exactly the number of
terminal empty lines required for parsing to reproduce the input string.

A string containing CR, a disallowed control character, or a line whose
representation would be ambiguous under the indentation rule uses one
double-quoted scalar with escaped `\n`/`\r` instead of block style.

Skill main content is serialized inline as `spec.content` under these scalar
rules. No generated sidecar exists.

## 7. Round-trip invariant

For every accepted AST `V`:

```text
parse_safe_yaml(canonical_emit(V)) == V
```

The parse uses the same alias/tag/merge and resource limits as lint. Canonical
emission of the parsed result must reproduce the exact original YAML bytes.

## 8. Golden strategy

Implementation adds immutable byte fixtures outside reference-only trees:

```text
tests/goldens/save/canonical/
├── assistant.yaml
├── workflow-marked.yaml
├── workflow-adoption-required.yaml
├── skill.yaml
├── datasource-git.yaml
├── datasource-svn.yaml
├── datasource-confluence.yaml
├── datasource-jira.yaml
├── datasource-xray.yaml
├── datasource-azure-devops-wiki.yaml
├── datasource-azure-devops-work-item.yaml
├── datasource-sharepoint.yaml
├── datasource-google.yaml
└── scalar-matrix.yaml
```

The scalar matrix covers every reserved plain token, empty strings, all
chomping modes, CR/LF, embedded tabs, controls, quotes/backslashes, Unicode
combining characters, supplementary scalars, Unicode key ordering, numeric
edges, null, and empty containers.

Each fixture has a normalized JSON input fixture and is tested for:

- exact bytes;
- safe parse equality;
- canonical re-emission equality;
- no BOM/CR/document marker/anchor/tag/alias/merge/trailing whitespace;
- exactly one document-final LF; and
- byte equality on Linux and every additional supported platform.

Changing any golden requires an explicit canonical-contract version review.
