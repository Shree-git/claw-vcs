# Multiple Agents Workflow

Use this path when several agents work on related intents.

- Give each agent a distinct registration name and local key.
- Keep each agent on a separate branch or workstream ref.
- Require each agent to create a change object linked to the owning intent.
- Use `claw sync push --dry-run` before updating shared remote refs.
- Integrate one ref at a time with `claw integrate --right <ref> --dry-run` before mutation.
- Prefer policies that require named checks and registered signer identities.

Conflict handling remains a human-owned step in v0.1.
