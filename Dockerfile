FROM rust:1-bookworm AS builder
WORKDIR /app

# Build dependency graph first to improve layer caching between source changes.
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/healthmaster /usr/local/bin/healthmaster
COPY config.toml /app/config.toml

ENTRYPOINT ["/usr/local/bin/healthmaster"]
CMD ["--config", "/app/config.toml"]
