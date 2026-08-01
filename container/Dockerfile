FROM rust:1.95.0-slim AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config ca-certificates nodejs npm \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY package.json package-lock.json tsconfig.json ./
COPY frontend ./frontend
COPY static ./static
RUN npm ci \
    && npm run build \
    && npm cache clean --force

COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

RUN cargo build --release --locked

FROM gcr.io/distroless/cc-debian13:nonroot AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/carteira-de-investimentos-maicaosa /usr/local/bin/carteira
COPY --from=builder /app/static ./static

ENV APP_BIND_ADDR=0.0.0.0:3000
EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/carteira"]
