# Intent, change, revision

Git starts with commits. Claw starts one level higher: the reason for the work.

```text
Intent
└── Change
    └── Revision
```

## Intent

An intent is the goal. It can include constraints, acceptance tests, status, and
links to later work. Intents are stored in the repository, so they travel with
history instead of living only in an external tracker.

Use intents for:

- product or operator goals
- acceptance tests that future changes must satisfy
- grouping related changes across agents or humans

## Change

A change is an implementation attempt linked to an intent. Several changes can
target the same intent when teams compare approaches or replace a failed attempt.

Use changes for:

- one branch of work toward an intent
- agent handoff between planning and implementation
- review of evidence before integration

## Revision

A revision is a recorded state of the repository. It has parents, author data,
patches, and links back through changes and intents.

Use revisions for:

- history traversal
- diff and merge
- Git import/export mapping
- policy checks at ship or integration time
