# syntax=docker/dockerfile:1.6
#
# Multi-stage Dockerfile for the open-ontologies (ontostar) MCP server.
#
# Build args:
#   FEATURES        Cargo feature list, space-separated. Default: "" (empty).
#                   Pass FEATURES=embeddings to enable ONNX/local embeddings.
#   RUST_VERSION    Rust toolchain. Default 1.75.
#
# Expected runtime mounts / env:
#   /data                 -> persistent state (Oxigraph DB, cache, snapshots).
#                            Configure via [general] data_dir = "/data".
#   /config/config.toml   -> canonical config file (TOML). Passed via
#                            `--config /config/config.toml` to `server serve_http`.
#   /secrets/.env         -> dotenv file containing GROQ_API_KEY and any other
#                            credentials. Loaded by dotenvy::dotenv() at start.
#                            Mount as a Kubernetes Secret or compose secret.
#
# The container exposes port 3050 by default. Override via CLI flag --port or
# config [http] port.
#
# Healthcheck: TCP probe on 3050 (no /health HTTP endpoint exists on the
# server today; the MCP transport listens on /mcp).

ARG RUST_VERSION=1.75

############################
# Stage 1 — builder
############################
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

ARG FEATURES=""

RUN apt-get update && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        libpq-dev \
        build-essential \
        clang \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_INCREMENTAL=0 \
    CARGO_PROFILE_RELEASE_DEBUG=0 \
    CARGO_TERM_COLOR=never

WORKDIR /build

# Copy the full workspace (workspace crates require sibling manifests).
COPY . .

RUN if [ -n "$FEATURES" ]; then \
        cargo build --release --features "$FEATURES" --bin open-ontologies; \
    else \
        cargo build --release --bin open-ontologies; \
    fi \
    && strip target/release/open-ontologies

############################
# Stage 2 — runtime
############################
FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="ontostar" \
      org.opencontainers.image.description="open-ontologies MCP server (ontostar)" \
      org.opencontainers.image.source="https://github.com/fabio-rovai/open-ontologies" \
      io.modelcontextprotocol.server.name="io.github.fabio-rovai/open-ontologies"

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        libpq5 \
        libgssapi-krb5-2 \
        libldap-2.5-0 \
        curl \
        netcat-openbsd \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 ontostar \
    && useradd  --system --uid 10001 --gid ontostar --home-dir /home/ontostar --create-home ontostar \
    && mkdir -p /data /config /secrets \
    && chown -R ontostar:ontostar /data /config /secrets

COPY --from=builder /build/target/release/open-ontologies /usr/local/bin/open-ontologies

USER ontostar
WORKDIR /home/ontostar

ENV OPEN_ONTOLOGIES_HTTP_HOST=0.0.0.0 \
    OPEN_ONTOLOGIES_HTTP_PORT=3050 \
    RUST_LOG=info

EXPOSE 3050

VOLUME ["/data", "/config", "/secrets"]

# TCP-level healthcheck — there is no /health HTTP endpoint today.
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD nc -z 127.0.0.1 3050 || exit 1

ENTRYPOINT ["/usr/local/bin/open-ontologies"]
CMD ["server", "serve_http", \
     "--config", "/config/config.toml", \
     "--host",   "0.0.0.0", \
     "--port",   "3050"]
