# `claw daemon`

Run the gRPC sync daemon and HTTP health listener.

```bash
claw daemon --listen 127.0.0.1:50051 --health-listen 127.0.0.1:50052
claw daemon --auth-token "$CLAW_TOKEN" --auth-role writer
claw daemon --rate-limit-per-minute 600 --max-push-chunk-bytes 8388608 --max-push-request-bytes 134217728
claw daemon --tls-cert server.pem --tls-key server-key.pem --client-ca-cert ca.pem
claw serve
```

In the production profile, non-local binds require authentication and TLS when the default hardened config is active. `claw serve` is an alias for `claw daemon`.
