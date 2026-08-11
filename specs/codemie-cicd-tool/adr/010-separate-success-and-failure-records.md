# ADR-010: Separate successful outcomes from safe failure diagnostics

## Status

Proposed

## Context

Earlier architecture used one outcome type for success and failure and carried
server error text/body after redaction. Product specification v27 instead
requires stdout to be empty on every failure and diagnostics to be constructed
only from an explicit allowlist. It also makes lint warnings part of the
successful per-file result only after complete repository-closure validation.
Arbitrary-redaction or streaming warnings during validation cannot meet that
boundary.

## Decision drivers

- Unambiguous CI streams
- Schema-enforced exclusion of payload/server/sensitive data
- Uniform behavior across text, JSON, debug, panic, and fatal paths
- Stable exit taxonomy independent of server prose

## Options considered

### A. One outcome schema with an optional error object

Rejected: encourages failure records on stdout and server-message attachment.

### B. Redact a rich internal error before printing

Rejected: non-allowlisted values can survive imperfect classification.

### C. Separate success outcome and allowlist diagnostic types

Selected.

## Decision

`contracts/outcome.schema.json` represents successful per-entity stdout only and
allows `valid|created|updated`. It contains only action and natural identity,
never a warning, error, server UUID, target URL, or request content.

`contracts/warning.schema.json` represents safe non-fatal stderr warnings and
contains only stable warning code/category plus source coordinates/field path.
Lint evaluates suspected-plaintext-secret and deprecated-value warnings only
after the complete discovered repository closure validates, and only for the
declaration selected by `--file`. It emits them in bytewise ascending fixed
warning-code order, then canonical-field-path order.

`contracts/diagnostic.schema.json` represents JSON stderr failures only. It is
closed and binds each stable `errorCode` to exactly one `category` and
`exitCode`; optional
members are limited to safe local coordinates, HTTP status/method/route
template, local request ID, and validated dedicated server
correlation ID.

Transport/domain errors are classified into safe enums before reaching output.
Raw bodies, response text, payloads, declaration values, arbitrary headers, and
exception strings are not inputs to the renderer. Text mode is rendered from
the same safe structure. If lint fails anywhere in the repository closure, the
warning sequence is discarded or never constructed and stderr contains exactly
the selected-output-mode failure diagnostic. Login success is handled by an
isolated token writer.

## Consequences

### Positive

- JSON Schema proves the machine-output boundary.
- stdout can be consumed only as successful data.
- Error classification remains stable across server releases.

### Negative

- Rich troubleshooting moves to correlation-based server logs.
- Batch processing stops without emitting partial success records if the
  invocation contract is single-entity.

### Risks

- Libraries may log before classification.
- A future field could accidentally broaden the allowlist.

## Follow-up actions

- Security-review the schema and renderer API.
- Enforce compile-time/private constructors around safe diagnostic fields.
- Add canary-value tests for all error and logging paths.

## References

- Product specification v27: FR-011/014/016/024/026, DR-006/009,
  QR-004/007/011
- ADR-003
