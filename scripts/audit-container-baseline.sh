#!/usr/bin/env bash
set -euo pipefail

IMAGE_TAG="${1:-carteira-investimentos:local}"

fail() {
  echo "ERRO: $*" >&2
  exit 1
}

expect_equal() {
  local label="$1"
  local actual="$2"
  local expected="$3"

  if [[ "${actual}" != "${expected}" ]]; then
    fail "${label}: esperado '${expected}', recebido '${actual}'"
  fi
}

expect_contains() {
  local label="$1"
  local actual="$2"
  local expected="$3"

  if [[ "${actual}" != *"${expected}"* ]]; then
    fail "${label}: valor esperado '${expected}' não encontrado em '${actual}'"
  fi
}

expect_absent_command() {
  local command_path="$1"
  shift

  if docker run --rm --entrypoint "${command_path}" "${IMAGE_TAG}" "$@" >/dev/null 2>&1; then
    fail "comando proibido encontrado na imagem runtime: ${command_path}"
  fi

  echo "OK comando ausente: ${command_path}"
}

if ! command -v docker >/dev/null 2>&1; then
  fail "docker não instalado ou indisponível no PATH"
fi

if ! docker image inspect "${IMAGE_TAG}" >/dev/null 2>&1; then
  fail "imagem '${IMAGE_TAG}' não encontrada; rode o build antes da auditoria"
fi

echo "== Docker inspect baseline =="

user="$(docker image inspect "${IMAGE_TAG}" --format '{{.Config.User}}')"
entrypoint="$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.Entrypoint}}')"
working_dir="$(docker image inspect "${IMAGE_TAG}" --format '{{.Config.WorkingDir}}')"
exposed_ports="$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.ExposedPorts}}')"
env_values="$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.Env}}')"
healthcheck="$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.Healthcheck}}')"
shell="$(docker image inspect "${IMAGE_TAG}" --format '{{json .Config.Shell}}')"

expect_equal "usuário runtime" "${user}" "65532:65532"
expect_equal "entrypoint" "${entrypoint}" '["/usr/local/bin/carteira"]'
expect_equal "working directory" "${working_dir}" "/app"
expect_contains "porta exposta" "${exposed_ports}" '"3000/tcp"'
expect_contains "bind público interno" "${env_values}" "APP_BIND_ADDR=0.0.0.0:3000"
expect_equal "healthcheck embutido" "${healthcheck}" "null"
expect_equal "shell configurado" "${shell}" "null"

echo "OK metadados públicos da imagem"

echo "== Runtime forbidden tools =="
expect_absent_command "/bin/sh" "-c" "true"
expect_absent_command "/bin/bash" "-c" "true"
expect_absent_command "/usr/bin/apt-get" "--version"
expect_absent_command "/usr/bin/npm" "--version"
expect_absent_command "/usr/bin/node" "--version"
expect_absent_command "/usr/local/cargo/bin/cargo" "--version"

echo "Baseline da imagem concluído sem desvios em ${IMAGE_TAG}."
