FROM rust:1.86-alpine AS builder
WORKDIR /src
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

COPY . .
RUN cargo build --release -p claw

FROM alpine:3.21
RUN addgroup -S claw -g 10001 && adduser -S claw -u 10001 -G claw
WORKDIR /app

COPY --from=builder /src/target/release/claw /usr/local/bin/claw
COPY crates/claw/deploy/container/probes/healthcheck.sh /usr/local/bin/healthcheck.sh
RUN chmod +x /usr/local/bin/claw /usr/local/bin/healthcheck.sh

USER 10001:10001
EXPOSE 50051 50052
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 CMD ["/usr/local/bin/healthcheck.sh"]

ENTRYPOINT ["/usr/local/bin/claw"]
CMD ["daemon", "--listen", "0.0.0.0:50051", "--health-listen", "0.0.0.0:50052"]
