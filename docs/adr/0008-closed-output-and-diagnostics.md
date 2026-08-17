# ADR-0008: Separate success outcomes from allowlist-only failure diagnostics

## Status

Accepted (originally ADR-010 and the allowlist-diagnostics core of ADR-003;
current behavior).

## Context

Earlier architecture used one outcome type for success and failure and carried
server error text/body after redaction. The product instead requires stdout to
be empty on every failure and diagnostics constructed only from an explicit
allowlist, with uniform behavior across text, JSON, debug, panic, and fatal
paths and a stable exit taxonomy independent of server prose.

## Decision

- `contracts/outcome-v2.schema.json` represents successful per-entity
  stdout only (`valid|created|updated|saved`) and contains only action,
  kind, project, and the applicable natural identity (plus
  `adoptionRequired` for an ID-selected unmarked Workflow). It never
  carries a warning, error, server UUID, target URL, or request content.
- `contracts/warning.schema.json` represents safe non-fatal stderr
  warnings (stable warning code/category plus source coordinates). Lint emits
  warnings only after the selected declaration validates, in fixed warning-code
  then canonical-field-path order. A failed declaration discards the warning
  sequence.
- `contracts/diagnostic-v2.schema.json` represents JSON stderr failures
  only. It is closed and binds each stable `errorCode` to exactly one
  `category` and `exitCode`; optional members are limited to safe
  local source coordinates, HTTP status/method/route template, local request
  ID, and validated server correlation ID.
- Transport/domain errors are classified into safe enums before reaching the
  renderer. Raw bodies, response text, payloads, declaration values, arbitrary
  headers, credentials, and exception strings are never inputs to output. The
  full internal error chain is additionally emitted at `DEBUG` level via
  `tracing` (opt-in via `RUST_LOG=debug`; stderr only; never part of
  the machine-readable stdout contract).
- Exit 0 is success; exit 1 is a reconciliation/server rejection reached after
  valid local input; exit 2 is usage, local validation, configuration,
  authentication/authorization, compatibility, connectivity, timeout, output,
  or internal failure.

## Consequences

- stdout can be consumed only as successful machine data; CI parsing is
  unambiguous.
- Error classification remains stable across server releases; rich
  troubleshooting moves to correlation-based server logs.
- A future field could accidentally broaden the allowlist; compile-time/private
  constructors and canary tests guard it.

## Alternatives considered

- One outcome schema with an optional error object: rejected (encourages
  failure records on stdout and server-message attachment).
- Redact a rich internal error before printing: rejected (non-allowlisted
  values can survive imperfect classification).

## References

- [outcome-v2.schema.json](../../contracts/outcome-v2.schema.json)
- [diagnostic-v2.schema.json](../../contracts/diagnostic-v2.schema.json)
- [warning.schema.json](../../contracts/warning.schema.json)
- `src/output.rs`, `src/render.rs`, `src/error.rs`
