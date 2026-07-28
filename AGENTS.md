# Repository Guidelines

## Project Structure & Module Organization

This repository is a Rust fullstack investment wallet for the DIO/Santander challenge. Core Rust code lives in `src/`, organized by responsibility:

- `src/domain/` — business rules and value validation.
- `src/application/` — use cases, errors, and orchestration.
- `src/infrastructure/` — database, configuration, security, telemetry, repositories.
- `src/presentation/` — HTTP routes, extractors, cookies, and web/API boundary.
- `migrations/` — SQLx PostgreSQL schema migrations.
- `specs/` — project requirements, decisions, routes, threats, and phased plans.
- `container/` and `docker-compose.dev.yml` — public container/dev database setup.

Do not version private VPS topology, Nginx, Fail2ban, firewall, deploy scripts, backups, or production secrets.

## Build, Test, and Development Commands

Use Rust 1.95.0 from `rust-toolchain.toml`.

```bash
cp .env.example .env
docker compose -f docker-compose.dev.yml up -d
sqlx migrate run
cargo run
```

Quality checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Coding Style & Naming Conventions

Follow standard `rustfmt` formatting. Use clear module names by layer and keep files cohesive without splitting every single function into its own file. Prefer explicit domain types for validated input, `Result<T, AppError>` at application boundaries, and structured logs without secrets.

Database migrations use timestamped SQLx names, for example `20260728000100_create_users.up.sql`.

## Testing Guidelines

Prioritize tests for authentication, authorization, financial calculations, validation, and database invariants. Unit tests should live beside the module under test. Integration tests should cover routes and persistence behavior when added.

Never use floating point types for money; financial calculations must use decimal-safe types and be covered by tests.

## Commit & Pull Request Guidelines

This repository does not yet define a long Git history convention. Use concise imperative commits such as `add auth session cleanup` or `create portfolio migrations`.

Pull requests should describe what changed, how it was tested, any migration impact, and whether security-sensitive behavior changed. Include screenshots when UI pages are modified.
