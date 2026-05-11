# ADR 0006: Capsules as Repository Objects

## Status

Accepted for v0.1.

## Context

Agent provenance loses value when it only exists in an external CI system or hosted platform database. Claw's core claim is offline-verifiable evidence attached to source history.

## Decision

Store capsules as first-class repository objects. A capsule records public provenance fields, optional encrypted private fields, revision binding, key identity, and signatures.

## Consequences

- Capsule verification can happen offline.
- Sensitive private fields must be encrypted before storage.
- Policy evaluation can inspect provenance without depending on a hosted service.
