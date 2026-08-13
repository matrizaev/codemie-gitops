# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in `codemie-gitops`, please **do not** create a public GitHub issue. Instead, please report it privately to the maintainers.

### How to Report

1. **Email**: matrizaev@gmail.com with the subject line: `[SECURITY] codemie-gitops vulnerability`

2. **Include the following information**:
   - Description of the vulnerability
   - Affected versions
   - Steps to reproduce (if possible)
   - Impact assessment (confidentiality, integrity, availability)
   - Any proof-of-concept or exploit code (if safe to share)

### Response Timeline

- **Acknowledgment**: Within 1-2 business days
- **Investigation**: We will assess the severity and impact
- **Fix**: Timeframe depends on complexity:
  - Critical (CVSS 9.0-10.0): Target 1-2 weeks
  - High (CVSS 7.0-8.9): Target 2-4 weeks
  - Medium (CVSS 4.0-6.9): Target 1-2 months
  - Low (CVSS 0.1-3.9): May be included in regular releases

- **Disclosure**: We will coordinate with you on responsible disclosure timing

## Security Considerations

### Authentication & Authorization

- **No built-in auth storage**: `codemie-gitops` does not store credentials or tokens. Authentication is delegated to the CodeMie server via OAuth.
- **Token handling**: Tokens are passed via `Authorization` header and should never be logged or printed.
- **Credential input**: Use environment variables or secure input methods; never embed credentials in code or config files.

### Transport Security

- **HTTPS only**: Always use HTTPS for `--url` in production.
- **TLS verification**: The client enforces certificate validation using `rustls`.
- **No insecure defaults**: HTTP is not supported for API communication.

### Input Validation

- **Schema validation first**: All YAML/JSON input is validated against JSON Schema before semantic processing.
- **Type safety**: Domain types enforce invariants through Rust's type system.
- **No arbitrary code execution**: The tool only performs CRUD operations on CodeMie entities; it does not execute scripts or arbitrary code.

### Dependency Security

- **Cargo audit**: Dependency vulnerabilities are checked in CI with `cargo audit`.
- **Locked dependencies**: `Cargo.lock` ensures reproducible builds.
- **Minimal dependencies**: The project uses only necessary, well-maintained crates.

Current critical dependencies:
- `tokio` — async runtime
- `reqwest` — HTTP client (with `rustls` for TLS)
- `serde` — serialization
- `thiserror` — error handling
- `tracing` — structured logging

### Logging & Observability

- **No credential logging**: Access tokens, session IDs, and sensitive data are never logged.
- **Structured logging**: Use `tracing` with explicit fields; avoid preformatted log strings with sensitive data.
- **Debug mode**: `RUST_LOG=debug` is available for troubleshooting but should not be used in production.

### Supply Chain Security

- **Build reproducibility**: Using locked dependencies and pinned Rust version.
- **Signed releases**: Binaries are published with checksums; GPG signatures may be added in the future.
- **Source transparency**: All code is open source and available for audit.

## Security Best Practices for Users

1. **Keep binaries up to date**: Use the latest released version to get security patches.
2. **Verify checksums**: Before running a downloaded binary, verify its SHA-256 against the published checksum.
3. **Use HTTPS**: Always specify `https://` URLs for the CodeMie server.
4. **Secure credential storage**: Store auth tokens in secure vaults (e.g., AWS Secrets Manager, HashiCorp Vault, GitHub Secrets).
5. **Rotate tokens regularly**: Follow your organization's credential rotation policy.
6. **Minimal permissions**: Request only the necessary scopes/permissions from your OAuth provider.
7. **Audit logs**: Enable and monitor CodeMie server audit logs for entity changes.

## Known Limitations

- **Stateless**: No local validation cache; each operation re-validates against the server.
- **Single-entity**: Process one entity per invocation; batch operations must be orchestrated externally.
- **No encryption at rest**: Credentials are not persisted locally; rely on server-side security.

## Compliance

- **License**: Apache License 2.0 (permissive, no warranties)
- **SLSA**: Future versions may provide SLSA supply-chain security levels
- **SBOMs**: Software Bill of Materials (SBOM) may be included in future releases

## Questions?

For security questions or clarifications, contact the maintainers privately at matrizaev@gmail.com.

---

**Last Updated**: August 2026
