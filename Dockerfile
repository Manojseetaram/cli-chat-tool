# ── Build stage ──────────────────────────────────────────
FROM rust:1.76-slim as builder

WORKDIR /app

# Cache dependencies first
COPY Cargo.toml Cargo.lock ./
COPY crates/relay/Cargo.toml  crates/relay/Cargo.toml
COPY crates/client/Cargo.toml crates/client/Cargo.toml

# Dummy src so cargo can resolve deps
RUN mkdir -p crates/relay/src crates/client/src && \
    echo "fn main(){}" > crates/relay/src/main.rs && \
    echo "fn main(){}" > crates/client/src/main.rs && \
    cargo build --release -p relay && \
    rm -rf crates/relay/src crates/client/src

# Now copy real source and build
COPY crates/ crates/
RUN touch crates/relay/src/main.rs && \
    cargo build --release -p relay

# ── Runtime stage ─────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/relay /usr/local/bin/relay

EXPOSE 3000

CMD ["relay"]