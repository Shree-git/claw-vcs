# ADR 0006: Capsules as Repository Objects

## Status

Accepted for v0.1.

## Metadata

- Date: 2026-05-12
- Owner: @Shree-git
- Supersedes: none
- Superseded by: none
- Related artifacts: `docs/concepts/capsules-and-evidence.md`, `docs/security/threat-model.md`, `crates/claw-crypto/src/capsule.rs`, `crates/claw-core/src/types/capsule.rs`

## Context

Agent provenance loses value when it only exists in an external CI system or hosted platform database. Claw's core claim is offline-verifiable evidence attached to source history.

## Decision

Store capsules as first-class repository objects. A capsule records public provenance fields, optional encrypted private fields, revision binding, key identity, and signatures.

## Alternatives Considered

- CI-hosted attestations only: useful for release artifacts, but not enough for source-history-level provenance.
- Git notes only: helpful bridge format, but not a native object model for policy, sync, and verification.
- External database: flexible, but loses offline verification and repository portability.

## Consequences

- Capsule verification can happen offline.
- Sensitive private fields must be encrypted before storage.
- Policy evaluation can inspect provenance without depending on a hosted service.

## Verification Links

- `docs/agents/evidence-schema.md`
- `docs/security/threat-model.md`
- `tests/vectors/capsules/signed_basic.json`
- `crates/claw-crypto/src/capsule.rs`
