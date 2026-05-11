# `claw patch`

Create and apply patch objects with registered codecs.

```bash
claw patch diff --old before.json --new after.json --codec json/tree
claw patch apply --base before.json --patch patch.json
```

Patch commands fail closed on unsupported codecs, invalid JSON for JSON codecs, and non-commutable patch streams.
