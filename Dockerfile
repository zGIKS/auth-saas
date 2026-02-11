# syntax=docker/dockerfile:1.7

FROM rust:1.88-bookworm AS builder
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --uid 10001 --create-home --shell /usr/sbin/nologin appuser

WORKDIR /app
COPY --from=builder /app/target/release/auth-service /usr/local/bin/auth-service

USER appuser:appuser

ENV PORT=8081
ENV RUST_LOG=info

EXPOSE 8081

ENTRYPOINT ["/usr/local/bin/auth-service"]
