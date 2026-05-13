# Capsules and evidence

A capsule is a signed record that says who or what produced a revision and what
checks ran before it was shipped.

Capsules are useful when humans and agents both write code. A Git author string
can say who committed a change, but it does not store a structured claim about
which agent, toolchain, or checks were involved. A Claw capsule records that data
as a repository object.

## Public fields

Public capsule fields can include:

- agent or signer identity
- agent version
- toolchain digest
- environment fingerprint
- evidence items such as test, lint, or security scan results

## Private fields

Sensitive metadata can be encrypted with XChaCha20-Poly1305. Use encrypted
private fields for data that should remain available to approved readers but
must not be public in normal repository views.

Recipient envelopes can wrap the capsule content key for policy-approved
recipient IDs. A policy that lists recipients requires matching envelopes before
ship or integration is allowed. A policy can also revoke recipient IDs; capsules
that still include those envelopes fail evaluation.

## Evidence

Evidence is a named result attached to a capsule. Policy can require evidence
before a change ships.

Example evidence names:

- `test`
- `lint`
- `security-scan`
- `policy-review`

Evidence should be specific and repeatable. Prefer command names or CI job names
that reviewers can find later.
