FROM rust:1.90-bookworm AS builder
WORKDIR /build
COPY . .
RUN cargo build --release -p coordinator -p signer-node

FROM debian:bookworm-slim AS coordinator
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/coordinator /usr/local/bin/coordinator
ENTRYPOINT ["coordinator"]

FROM debian:bookworm-slim AS signer-node
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/signer-node /usr/local/bin/signer-node
ENTRYPOINT ["signer-node"]
