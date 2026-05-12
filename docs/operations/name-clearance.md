# Name Clearance Runbook

This runbook records the owner-side checks for the public `Claw VCS` name.
It is not legal advice. Treat ambiguous results as a reason to ask counsel
before investing in a permanent logo, domain, package identity, or launch post.

## Scope

Check these launch identifiers:

- Product name: `Claw VCS`
- Repository name: `claw-vcs`
- CLI name: `claw`
- Cargo package names: `claw-vcs` and `claw-vcs-*`
- Suggested social handle stem: `clawvcs`
- Suggested domain stem: `clawvcs`

## Trademark Search

Search at least:

- USPTO Trademark Search: <https://www.uspto.gov/trademarks/search>
- WIPO Global Brand Database: <https://branddb.wipo.int/>
- EUIPO eSearch: <https://euipo.europa.eu/eSearch/>

Search terms:

- `Claw`
- `Claw VCS`
- `Claw version control`
- `claw-vcs`
- `clawvcs`

Record:

- search date
- jurisdiction/database
- exact search term
- similar marks found
- owner
- goods/services class
- relevance to developer tools, version control, AI agents, security, or software
- decision: clear, watch, blocked, or counsel review required

## Domain Search

Use registrar searches plus ICANN Lookup: <https://lookup.icann.org/>.

Check at least:

- `clawvcs.com`
- `clawvcs.dev`
- `clawvcs.io`
- `claw-vcs.com`
- `claw-vcs.dev`
- `claw-vcs.io`

Record:

- availability
- registrar
- price or renewal risk
- registrant visibility, if already registered
- whether the domain could be confused with an existing developer tool

## Social Handles

Check the intended launch channels directly because availability changes:

- GitHub organization/user namespace
- X/Twitter
- Bluesky
- Mastodon
- LinkedIn page
- YouTube

Record:

- handle checked
- availability
- squatter/confusion risk
- whether the handle was reserved
- account owner and recovery email location

## Package Names

Run the repo preflight and crates.io dry-run helper:

```bash
scripts/public-launch-preflight.sh
scripts/publish-cratesio.sh
```

Before broad announcement, reserve or publish:

```text
claw-vcs
claw-vcs-core
claw-vcs-store
claw-vcs-patch
claw-vcs-merge
claw-vcs-crypto
claw-vcs-policy
claw-vcs-sync
claw-vcs-git
```

Record the crates.io owner account and recovery path. Do not document
`cargo install claw-vcs` until the package set is live and verified.

Copy the template:

```bash
cp docs/operations/name-clearance-evidence.template.md docs/operations/name-clearance-evidence.md
```

Save completed launch evidence in:

```text
docs/operations/name-clearance-evidence.md
```

If evidence is stored elsewhere, run strict preflight with:

```bash
CLAW_PREFLIGHT_STRICT=1 \
  CLAW_PREFLIGHT_NAME_EVIDENCE=/path/to/evidence.md \
  CLAW_PREFLIGHT_CRATESIO_OWNER=<owner> \
  scripts/public-launch-preflight.sh
```

## GitHub Social Preview

Upload `docs/assets/social-preview.png` in repository settings. GitHub documents
custom social previews in its repository customization docs:
<https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/customizing-your-repositorys-social-media-preview>.

Verify after upload:

```bash
scripts/public-launch-preflight.sh
```

The preflight checks `usesCustomOpenGraphImage` and warns until the image is
uploaded.

## Evidence Template

Use [name-clearance-evidence.template.md](name-clearance-evidence.template.md)
as the source template. Copy supporting links, screenshots, or account records
into the launch issue when they do not belong in the repository.

Strict preflight treats the evidence as complete only when the final decision,
domain, social-handle, crates.io, and social-preview upload fields are filled
in with non-placeholder values, and `GitHub social preview uploaded` is `yes`.
