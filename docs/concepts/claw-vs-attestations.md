# Claw VCS, SLSA, GitHub Attestations, and Sigstore

These systems overlap, but they operate at different layers.

| System | Primary claim |
|---|---|
| SLSA provenance | How a build artifact was produced. |
| GitHub artifact attestations | Which GitHub workflow produced an artifact for a repository and commit. |
| Sigstore/Cosign | A signature and transparency-log-backed identity story for artifacts or blobs. |
| Claw VCS | Source-history-level provenance: intent, change, revision, capsule, evidence, and policy as repository objects. |

Claw VCS should use release attestations and Sigstore for its own binaries. Inside a Claw repository, capsules and policies answer a different question: what evidence was claimed for this source revision, which key signed it, and did repository policy allow it?

```mermaid
flowchart LR
  Agent["Agent"] -->|signs| Capsule["Capsule"]
  Capsule --> Evidence["Evidence"]
  Evidence --> Policy["Policy"]
  Policy -->|allows or blocks| Integrate["Integrate"]
  Integrate --> Revision["Revision"]
```

Attestations and signatures do not remove the need to reason about the trust boundary. A Claw repository can preserve and verify signed claims, but those claims still depend on agent keys, runner integrity, remote object integrity, and policy strength.

```mermaid
flowchart TD
  Agent["Agent key"] --> Capsule["Signed capsule"]
  Runner["Runner integrity"] --> Evidence["Evidence"]
  Remote["Remote refs and objects"] --> Revision["Revision"]
  Policy["Repository policy"] --> Decision["Integration decision"]
  Capsule --> Decision
  Evidence --> Decision
  Revision --> Decision
  BadKey["Compromised key"] -. "can sign bad claims" .-> Capsule
  BadRunner["Compromised runner"] -. "can produce bad evidence" .-> Evidence
  BadRemote["Tampered remote"] -. "can hide or reorder state" .-> Revision
  WeakPolicy["Weak policy"] -. "can allow weak evidence" .-> Decision
```
