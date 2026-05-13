# Telemetry Policy

Claw VCS does not collect or transmit product telemetry by default.

Local daemon deployments may emit local logs, metrics, and traces when configured by the operator. Those signals stay in the operator's environment unless the operator exports them to their own observability system.

Do not put secrets, bearer tokens, private keys, customer data, or plaintext encrypted-private capsule fields in logs, metrics, traces, support bundles, screenshots, or public issues.

If hosted ClawLab-style services become available later, they must publish a separate privacy policy, data retention policy, security model, and telemetry opt-in/opt-out behavior before being treated as live.
