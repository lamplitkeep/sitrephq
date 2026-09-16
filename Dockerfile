FROM rust:1-slim-bookworm AS build
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY sitrep/Cargo.toml sitrep/
COPY sitrep-agent/Cargo.toml sitrep-agent/
RUN mkdir sitrep/src sitrep-agent/src \
    && echo "fn main() {}" > sitrep/src/main.rs \
    && echo "fn main() {}" > sitrep-agent/src/main.rs \
    && cargo build --release -p sitrephq || true
COPY . .
RUN touch sitrep/src/main.rs && cargo build --release -p sitrephq


# --- Full (Shell support)
FROM debian:13-slim AS full
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    curl jq \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/target/release/sitrep /sitrep
EXPOSE 1986
ENTRYPOINT ["/sitrep", "/config/config.yml"]

# --- Minimal (DEFAULT)
FROM gcr.io/distroless/cc-debian13 AS minimal
COPY --from=build /build/target/release/sitrep /sitrep
EXPOSE 1986
ENTRYPOINT ["/sitrep", "/config/config.yml"]