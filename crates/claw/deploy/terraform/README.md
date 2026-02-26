# Claw Helm deployment (Terraform reference)

This directory is a minimal Terraform reference for installing the local Claw Helm chart via `helm_release`.

## Usage

```bash
terraform init
terraform apply
```

## Common overrides

Set variables at apply time:

```bash
terraform apply \
  -var='namespace=claw-system' \
  -var='release_name=claw' \
  -var='image_repository=ghcr.io/infinite-apps/claw' \
  -var='image_tag=v0.1.0' \
  -var='values_files=["../helm/claw/values.yaml"]'
```

## Notes

- `chart_path` defaults to `../helm/claw` in this repository.
- The example intentionally stays minimal and is intended as a deployment reference.
