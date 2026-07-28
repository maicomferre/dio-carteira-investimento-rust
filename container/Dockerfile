FROM rust:1.95.0-slim AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --locked

FROM debian:trixie-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --home-dir /nonexistent --shell /usr/sbin/nologin carteira

WORKDIR /app

COPY --from=builder /app/target/release/carteira-de-investimentos-maicaosa /usr/local/bin/carteira

USER 10001:10001

ENV APP_BIND_ADDR=0.0.0.0:3000
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:3000/health/live"]

ENTRYPOINT ["/usr/local/bin/carteira"]
