#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${PROJECT_ROOT}/reports/security"
mkdir -p "${REPORT_DIR}"

cd "${PROJECT_ROOT}"

echo "== Rust advisory audit =="
if command -v cargo-audit >/dev/null 2>&1; then
  cargo audit
else
  echo "cargo-audit não instalado; instale com: cargo install cargo-audit --locked"
fi

echo "== npm advisory audit =="
npm audit --audit-level=moderate

echo "== Trivy filesystem scan =="
if command -v trivy >/dev/null 2>&1; then
  trivy fs \
    --scanners vuln,secret,misconfig \
    --skip-dirs target \
    --skip-dirs node_modules \
    --skip-dirs .git \
    --severity HIGH,CRITICAL \
    --exit-code 1 \
    .

  trivy fs \
    --format cyclonedx \
    --output "${REPORT_DIR}/sbom.cdx.json" \
    --skip-dirs target \
    --skip-dirs node_modules \
    --skip-dirs .git \
    .
  echo "SBOM gerado em reports/security/sbom.cdx.json (ignorado pelo Git)."
else
  echo "trivy não instalado; instale para scan de filesystem e geração de SBOM."
fi

echo "== Secret filename guard =="
if find . \
  -path './.git' -prune -o \
  -path './target' -prune -o \
  -path './node_modules' -prune -o \
  -path './static/vendor' -prune -o \
  \( \
    -name '.env' -o \
    -name '*.pem' -o \
    -name '*.key' -o \
    -name '*secret*' -o \
    -name '*credential*' \
  \) -print | grep -q .; then
  echo "Arquivos com nome sensível encontrados; revise antes de commitar."
  exit 1
fi

echo "Auditoria concluída."
