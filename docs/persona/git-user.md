# Git User

Start with:

- `docs/migration/from-git.md`
- `docs/workflows/git-interop.md`
- `examples/git-roundtrip/README.md`

Use Claw beside Git while evaluating v0.1:

```bash
claw init
claw git-import --git-dir .git --git-ref refs/heads/main --ref-name heads/main
claw git-export --git-dir /tmp/exported.git --ref-name heads/main --branch claw/main
claw git-roundtrip
```

Known caveats: tags, submodules, hooks, and some Git metadata are not first-class Claw objects in v0.1.
