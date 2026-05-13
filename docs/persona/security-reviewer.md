# Security Reviewer

Start with:

- `docs/security/threat-model.md`
- `docs/reference/security.md`
- `docs/security/verifying-releases.md`
- `docs/reference/known-limitations.md`
- `.github/workflows/`

Review focus:

- workflow permissions and pinned actions
- release signatures, attestations, and SBOM
- capsule claims versus trust assumptions
- daemon auth, TLS, replay protection, authorization, and audit logs
- secret scanning status and dependency policy results
