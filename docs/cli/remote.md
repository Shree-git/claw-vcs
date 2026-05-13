# `claw remote`

Manage configured remotes.

```bash
claw remote list
claw remote list --json
claw remote add origin http://127.0.0.1:50051 --kind grpc --dry-run
claw remote remove origin --dry-run
```

Remote entries can point at self-hosted gRPC daemons or planned hosted ClawLab-style remotes.
