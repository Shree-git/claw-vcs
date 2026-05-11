# Roadmap

This roadmap tracks public launch hardening for Claw VCS. It is a planning
guide, not a date promise.

## v0.1

- Harden the `v0.1.x` self-hosted operator path.
- Local repo, object model, snapshots, capsules, basic policies, and Git bridge.
- Keep release artifacts signed and verifiable.
- Keep daemon health, metrics, backup, restore, and rollback docs current.
- Document public interfaces, stability levels, and data layout.
- Improve issue triage, PR review, and maintainer handoff docs.
- Keep primitive docs complete for all 12 object types.
- Keep the basic demo script smoke-testable against the current CLI.
- Publish small versioned demo media for docs and social previews.

## v0.2

- Evidence schema hardening.
- Expand Git interop coverage and document tested bridge cases.
- Internal event bus coverage for all mutation paths.
- Add more migration examples for teams moving from Git-only workflows.
- Publish agent integration examples that create intents, changes, and capsules.
- Add more policy examples for review, sensitive paths, and evidence checks.
- Make compatibility test output easier for operators to read.
- Expose a supported CLI path for attaching policy refs to intents.
- Add explicit agent key import, rotation, and revocation workflows if they are
  promoted into the CLI.

## v0.3

- Remote sync hardening and compatibility matrix expansion.
- Recipient-based capsule encryption and private-field read authorization.
- Broaden hosted remote integration once the self-hosted path is proven.
- Add more codecs for structured file formats.
- Publish larger scale benchmarks and sizing guidance.
- Add deeper admin tooling for repository inspection and repair.

## v1.0

- Stable object format.
- Stable sync protocol.
- Documented migration path for pre-1.0 repositories.
- Production-ready daemon deployment profile.

## Not planned for `v0.1.x`

- Replacing Git for every project type.
- Silent public-interface removals.
- Support for unpinned mixed-version fleets outside documented rollout windows.
- Claiming global policy enforcement for policies that are not referenced by
  intents.
- Claiming hosted remote availability unless a release note names it.
