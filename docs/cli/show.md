# `claw show`

Inspect an object by ref, hex object ID, or `clw_` display ID.

```bash
claw show heads/main
claw show --json heads/main
claw show <capsule-id> --decrypt-private --recipient security-reviewer --recipient-secret-key ./security-reviewer.x25519
```

JSON output includes the object display ID, hex ID, object type, and serialized object value.

For capsule objects, human output includes recipient envelope IDs. The
`--decrypt-private` path decrypts encrypted private capsule fields with the
matching X25519 recipient secret key.
