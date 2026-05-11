# `claw sync`

Pull from or push to configured remotes.

```bash
claw sync pull --remote origin --ref-name heads/main
claw sync push --remote origin --ref-name heads/main
claw sync push --remote origin --ref-name heads/main --dry-run
claw sync clone <remote> <path>
```

`sync push --dry-run` connects to the remote, resolves the local and remote refs, and reports the object upload/ref update that would occur without mutating the remote.

Production remotes should require authentication and TLS. Use the sync-level TLS flags before the subcommand:

```bash
claw sync \
  --tls-ca-cert ./ca.pem \
  --tls-domain claw.example.com \
  --client-cert ./client.pem \
  --client-key ./client-key.pem \
  push --remote https://claw.example.com:50051
```

`--client-cert` and `--client-key` must be provided together. Protocol negotiation failures should be treated as compatibility issues, not retried blindly.
