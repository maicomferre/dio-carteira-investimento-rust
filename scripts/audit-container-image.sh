#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="${PROJECT_ROOT}/reports/security"
IMAGE_TAG="${1:-carteira-investimentos:local}"
IMAGE_TAR="${REPORT_DIR}/carteira-investimentos-local.tar"

mkdir -p "${REPORT_DIR}"

cd "${PROJECT_ROOT}"

if ! command -v docker >/dev/null 2>&1; then
  echo "docker não instalado ou indisponível no PATH."
  exit 1
fi

if ! command -v trivy >/dev/null 2>&1; then
  echo "trivy não instalado; instale para escanear a imagem."
  exit 1
fi

echo "== Docker image build =="
docker build -f container/Dockerfile -t "${IMAGE_TAG}" .

echo "== Docker image export =="
docker save "${IMAGE_TAG}" -o "${IMAGE_TAR}"

echo "== Trivy image scan =="
trivy image \
  --input "${IMAGE_TAR}" \
  --scanners vuln,secret,misconfig \
  --severity HIGH,CRITICAL \
  --exit-code 1

./scripts/audit-container-baseline.sh "${IMAGE_TAG}"

echo "Scan da imagem concluído sem HIGH/CRITICAL em ${IMAGE_TAG}."
