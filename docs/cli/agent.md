# `claw agent`

Manage local agent registration records and signing keys.

```bash
claw agent register --name ci-agent
claw agent list
claw agent status ci-agent
```

Agent private keys are stored outside the repository under the user Claw home. Revocation is currently handled through policy and operational denylist process rather than a first-class `agent revoke` command.
