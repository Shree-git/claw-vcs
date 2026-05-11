# `claw auth`

Manage saved authentication profiles for hosted or daemon-backed remotes.

```bash
claw auth login --profile prod
claw auth token set <token> --profile prod
claw auth token show --profile prod
claw auth token list
claw auth logout --profile prod
```

Tokens are local credential material. Do not commit `~/.claw/auth.toml`, `~/.claw/auth.key`, copied bearer tokens, or support bundles containing auth data.
