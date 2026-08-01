#!/usr/bin/env bash
set -euo pipefail

if [ -f ./.env ]; then
  set -a
  # shellcheck disable=SC1091
  . ./.env
  set +a
fi

if [ -z "${DATABASE_MIGRATION_URL:-}" ]; then
  echo "DATABASE_MIGRATION_URL é obrigatório para migrations." >&2
  echo "Use .env.example como referência e não rode migrations com DATABASE_URL runtime." >&2
  exit 1
fi

DATABASE_URL="${DATABASE_MIGRATION_URL}" sqlx migrate run
