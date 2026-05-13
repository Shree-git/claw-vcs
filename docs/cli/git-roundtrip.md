# `claw git-roundtrip`

Verify Claw to Git to Claw bridge integrity for a ref.

```bash
claw git-roundtrip
claw git-roundtrip --ref-name heads/main
```

The command is a release-readiness smoke test for Git export/import plumbing. Use it alongside real Git checks such as `git fsck`, `git log`, and checkout tests.
