# Git Roundtrip

```bash
claw init
printf 'hello\n' > hello.txt
claw snapshot -m "initial"
claw git-export --git-dir /tmp/claw-export.git --all-heads
git -C /tmp/claw-export.git fsck
git -C /tmp/claw-export.git log --oneline
claw git-import --git-dir /tmp/claw-export.git --all-branches
claw git-roundtrip
```

Use real Git checks before trusting exported repositories.
