# ╔══════════════════════════════════════════════════════════════════════════╗
# ║  VIVA relay — production Dockerfile                                      ║
# ║  Deploys the relay server only (client is a local binary).               ║
# ║                                                                          ║
# ║  Build:  docker build -t viva-relay .                                    ║
# ║  Run:    docker run -e MONGO_URI=... -e PORT=3000 -p 3000:3000 viva-relay║
# ╚══════════════════════════════════════════════════════════════════════════╝

# ── Stage 1: Build ────────────────────────────────────────────────────────────
FROM rust:1.76-slim AS builder

# Install build dependencies (needed for OpenSSL / TLS)
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/relay/Cargo.toml  crates/relay/Cargo.toml
COPY crates/client/Cargo.toml crates/client/Cargo.toml

# Build with dummy sources so dependencies are cached in a separate layer
RUN mkdir -p crates/relay/src crates/client/src && \
    echo "fn main(){}" > crates/relay/src/main.rs  && \
    echo "fn main(){}" > crates/client/src/main.rs && \
    cargo build --release -p relay 2>&1 | tail -5 && \
    rm -rf crates/relay/src crates/client/src

# Copy real sources and do the actual build
COPY crates/ crates/
# Touch so cargo knows the sources changed
RUN touch crates/relay/src/main.rs && \
    cargo build --release -p relay

# ── Stage 2: Minimal runtime image ───────────────────────────────────────────
FROM debian:bookworm-slim

# libssl3 for TLS connections to MongoDB Atlas (wss://)
RUN apt-get update && \
    apt-get install -y libssl3 ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/relay /usr/local/bin/relay

# Render sets PORT automatically; default to 3000 locally
ENV PORT=3002

EXPOSE $PORT

# Healthcheck so Render knows the container is alive
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:${PORT}/health || exit 1

CMD ["relay"]