# Verifying Releases

Use these checks before installing a downloaded Claw VCS release artifact.

## Expected Identity

- Repository: `Shree-git/claw-vcs`
- Binary name: `claw`
- Release tags: `vX.Y.Z`
- Artifact signing: Sigstore/Cosign when release assets provide signatures
- Provenance: GitHub artifact attestations when release assets provide attestations

## Checksums

Download the archive and checksum file from the same GitHub Release:

```bash
sha256sum -c SHA256SUMS
```

On macOS:

```bash
shasum -a 256 -c SHA256SUMS
```

## Cosign Blob Signatures

When a release provides `.sig` and certificate material:

```bash
cosign verify-blob \
  --certificate ./claw.tar.gz.pem \
  --signature ./claw.tar.gz.sig \
  ./claw.tar.gz
```

Check the certificate identity and issuer match the release workflow documented for this repository.

## GitHub Artifact Attestations

When a release provides GitHub artifact attestations:

```bash
gh attestation verify ./claw.tar.gz --repo Shree-git/claw-vcs
```

Verify the attestation references the expected repository, commit SHA, workflow, and tag.

## SBOM

When a release includes SPDX or CycloneDX SBOMs:

```bash
jq '.name, .packages | length' claw-v0.1.0.sbom.spdx.json
```

Use the SBOM to review dependency inventory and compare it with the release commit.

## Install Smoke Test

After installation:

```bash
claw --version
claw doctor
mkdir -p /tmp/claw-demo
cd /tmp/claw-demo
claw init
claw status
```

## If Verification Fails

- Do not run the binary.
- Delete the artifact and re-download from the GitHub Release.
- Confirm you are using the expected repository and tag.
- Open a security advisory if the signature, checksum, or attestation still does not match.
