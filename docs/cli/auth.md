# `claw auth`

Manage saved authentication profiles for hosted or daemon-backed remotes.

```bash
claw auth login --base-url https://auth.example.invalid --profile prod
claw auth token set <token> --base-url https://daemon.example.invalid --profile prod
claw auth token show --profile prod
claw auth token list
claw auth logout --profile prod
```

Hosted-service auth is not configured by default in v0.1. Pass `--base-url`
for a self-hosted endpoint or for a hosted service that release notes explicitly
mark as live.

Tokens are local credential material. Do not commit `~/.claw/auth.toml`, `~/.claw/auth.key`, copied bearer tokens, or support bundles containing auth data.
