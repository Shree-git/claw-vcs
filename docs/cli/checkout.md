# `claw checkout`

Switch branches or materialize a revision into the working tree.

```bash
claw checkout heads/main
claw checkout <revision-id>
claw checkout --json heads/main
claw checkout --dry-run heads/main
```

Checkout refuses invalid refs and reports remediation through the standard CLI error envelope when `--error-format json` is used.
