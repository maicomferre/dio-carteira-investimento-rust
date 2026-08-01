#!/usr/bin/env bash
set -euo pipefail

failures=0

fail() {
  echo "ERRO: $*" >&2
  failures=$((failures + 1))
}

tracked_and_pending_files() {
  {
    git ls-files
    git ls-files --others --exclude-standard
  } | sort -u
}

is_allowed_file() {
  case "$1" in
    .env.example) return 0 ;;
    scripts/check-public-boundary.sh) return 0 ;;
    scripts/audit-container-image.sh) return 0 ;;
    scripts/audit-container-baseline.sh) return 0 ;;
    scripts/audit-supply-chain.sh) return 0 ;;
    specs/*) return 0 ;;
    README.md|AGENTS.md|SPEC.md) return 0 ;;
    *) return 1 ;;
  esac
}

echo "== Public repository boundary =="

while IFS= read -r file; do
  [[ -z "${file}" ]] && continue

  case "${file}" in
    .env|.env.*)
      if [[ "${file}" != ".env.example" ]]; then
        fail "arquivo de ambiente real não pode ser versionado: ${file}"
      fi
      ;;
    *.pem|*.key|*.crt|*.p12|*.pfx|*.kubeconfig|*.ovpn)
      fail "arquivo com material sensível não pode ser versionado: ${file}"
      ;;
    infra/*|deploy/*|backups/*|private/*)
      fail "diretório privado de infraestrutura não pode ser versionado: ${file}"
      ;;
    scripts/nginx/*|scripts/security/*|scripts/ops/*)
      fail "scripts privados de host/deploy não podem ser versionados: ${file}"
      ;;
    scripts/docker/.env*|scripts/docker/*token*|scripts/docker/*deploy*|scripts/docker/*cron*)
      fail "artefato privado de Docker/host não pode ser versionado: ${file}"
      ;;
    scripts/*deploy*|scripts/*release*|scripts/*vps*|scripts/*fail2ban*|scripts/*nginx*|scripts/*firewall*|scripts/*sysctl*)
      fail "script operacional privado não pode ser versionado: ${file}"
      ;;
  esac
done < <(tracked_and_pending_files)

echo "== Public content pattern guard =="

content_files=()
while IFS= read -r file; do
  [[ -z "${file}" ]] && continue
  [[ -f "${file}" ]] || continue
  is_allowed_file "${file}" && continue

  case "${file}" in
    package-lock.json|Cargo.lock) continue ;;
    *.png|*.jpg|*.jpeg|*.gif|*.webp|*.ico|*.woff|*.woff2|*.tar|*.gz|*.zip) continue ;;
  esac

  content_files+=("${file}")
done < <(tracked_and_pending_files)

if ((${#content_files[@]} > 0)); then
  if grep -EIn \
    -e '(^|[^[:alnum:]_])(server_name|proxy_pass|upstream[[:space:]]+[[:alnum:]_-]+|fail2ban|jail\.local|nftables|iptables|ufw|firewalld|certbot|letsencrypt)([^[:alnum:]_]|$)' \
    -e '(^|[^[:alnum:]_])(ssh|scp|rsync)[[:space:]]+[^[:space:]]+@' \
    -e '(/etc/nginx|/etc/fail2ban|/etc/letsencrypt|/var/www/|/var/log/nginx|/run/docker\.sock)' \
    -e '(BEGIN (RSA|OPENSSH|EC|DSA|PRIVATE) KEY)' \
    "${content_files[@]}"; then
    fail "padrões privados de infraestrutura/segredo encontrados em arquivos públicos"
  fi
fi

if ((failures > 0)); then
  echo "Gate público reprovado com ${failures} problema(s)." >&2
  exit 1
fi

echo "Gate público aprovado: nenhum artefato privado de VPS/host/deploy detectado."
