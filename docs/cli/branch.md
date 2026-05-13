# `claw branch`

List, create, and delete Claw refs under `heads/`.

```bash
claw branch --json
claw branch create feature/example
claw branch create feature/example --dry-run
claw branch delete feature/example --dry-run
```

Deletion and creation dry runs report the target ref mutation without writing the ref store.
