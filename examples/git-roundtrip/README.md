# Git Roundtrip

```bash
claw init
printf 'hello\n' > hello.txt
claw snapshot -m "initial"
claw git-export /tmp/claw-export.git --all-heads
git -C /tmp/claw-export.git fsck
git -C /tmp/claw-export.git log --oneline
claw git-import /tmp/claw-export.git
claw git-roundtrip
```

Use real Git checks before trusting exported repositories.
