# Concepts

Claw stores code history plus the intent, evidence, and policy data around that
history. Start here if you know Git and want the Claw object model.

## Pages

- [Intent, change, revision](intent-change-revision.md)
- [Capsules and evidence](capsules-and-evidence.md)
- [Policies](policies.md)
- [Object model](object-model.md)
- [Claw VCS vs attestations](claw-vs-attestations.md)
- [Agent honesty is not enough](agent-honesty-is-not-enough.md)

## Short glossary

| Term | Meaning |
|---|---|
| Intent | The goal for work, with constraints and acceptance tests. |
| Change | One attempt to satisfy an intent. |
| Revision | A recorded repository state, similar to a commit. |
| Capsule | Signed provenance and evidence for a revision. |
| Policy | Versioned rules that gate shipping and integration. |
| Workstream | An ordered stack of related changes. |
