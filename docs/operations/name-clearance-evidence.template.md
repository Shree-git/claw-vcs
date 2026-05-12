# Name Clearance Evidence

Use this file as the source template for the strict launch evidence file.
Do not rename it to `name-clearance-evidence.md` until the owner-side checks are
complete and the fields below contain real evidence.

- Date:
- Reviewer:
- Trademark databases checked:
- Similar marks and disposition:
- Domains checked/reserved:
- Social handles checked/reserved:
- crates.io packages reserved/published:
- GitHub social preview uploaded: no
- Counsel review required: yes/no
- Final decision:

## Notes

- Record links, screenshots, or account records in the launch issue if they do
  not belong in the repository.
- Strict preflight requires completed domain, social-handle, crates.io,
  trademark-search, similar-mark, reviewer, and final decision fields, plus
  `Date` in `YYYY-MM-DD` format, `Counsel review required: yes/no`, and
  `GitHub social preview uploaded: yes`. Placeholder values such as `pending`,
  `TBD`, `unknown`, or `not complete` do not pass.
- Validate the completed file before launch:

  ```bash
  scripts/verify-name-clearance-evidence.sh docs/operations/name-clearance-evidence.md
  ```
