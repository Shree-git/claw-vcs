# `claw completions`

Generate shell completion scripts.

```bash
claw completions bash
claw completions zsh
claw completions fish
claw completions powershell
claw completions elvish
```

Completion scripts are generated from the same Clap command definition used by
the CLI parser, so command and flag coverage stays aligned with `claw --help`.

`claw completion <shell>` is accepted as a compatibility alias.
