# Policy-gated ship workflow

Use this flow when a repository requires evidence before changes can ship or be
integrated.

## Current CLI boundary

`claw policy create`, `claw policy list`, and `claw policy show` manage policy
objects. Enforcement happens only for policies referenced by the intent being
shipped or integrated. The current CLI can create intents, but it does not yet
expose a command to edit an intent's `policy_refs` field. Until that command
exists, enforced policy-gated demos need an integration/API/import step that
pre-populates `policy_refs`.

Do not assume a policy named `default` applies automatically.

## 1. Create or read policy

List policies:

```bash
claw policy list
```

Inspect the policy object:

```bash
claw policy show <policy-id>
```

Create the common CI policy shape:

```bash
claw policy create \
  --id ci-required \
  --check test \
  --check lint \
  --reviewer release-bot \
  --min-trust-score 80% \
  --require-fresh-evidence \
  --trusted-runner github-actions/release \
  --evidence-max-age-ms 86400000
```

## 2. Run checks

Run the commands required by policy. Common names are:

- `test`
- `lint`
- `security-scan`
- `policy-review`

Keep evidence names stable across local runs and CI.

## 3. Ship with evidence

```bash
claw ship \
  --intent <intent-id> \
  --revision-ref heads/release-candidate \
  --agent release-bot \
  --evidence test=pass \
  --evidence lint=pass \
  --evidence-command "cargo test --workspace --all-targets" \
  --runner github-actions/release \
  --environment-digest sha256:<toolchain-digest> \
  --log-digest sha256:<log-digest> \
  --evidence-expires-in-ms 86400000 \
  --private-file private-capsule.json \
  --recipient-key security-reviewer:security-key:<hex-x25519-public-key>
```

Use `--co-sign <agent-id>` when the policy requires additional reviewer
signatures. `required_reviewers` can match registered agent IDs or public key
IDs from capsule signatures.

## 4. Integrate a branch

`claw integrate` requires the right side explicitly:

```bash
claw checkout main
claw integrate --right heads/release-candidate --dry-run
claw integrate --right heads/release-candidate
```

During integration, Claw checks applicable revisions from the right side that
are not already reachable from the left side. For each policy-referenced
revision, it verifies capsule signatures against registered agents and evaluates
the referenced policies.

## 5. Troubleshoot denials

Common denial causes:

- the intent has no `policy_refs`, so the expected policy is not actually in the
  enforcement path
- required evidence name is missing or not `pass`
- signer does not match a registered agent identity
- `--co-sign` is missing for required reviewer policies
- sensitive path policy needs encrypted private capsule metadata or authorized
  recipient envelopes that are not present on the capsule
- `min_trust_score` is higher than the pass ratio of capsule evidence
- freshness policy requires revision-bound evidence fields that were omitted

For sensitive paths, the policy can require encrypted capsule private fields or
quarantine. Treat a policy denial as a real block, not a warning to bypass.
