# Fase 1 — Fundação segura e observável

- **Status:** em execução
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-08-01

## Objetivo

Entregar um esqueleto Linux reproduzível, com limites arquiteturais, PostgreSQL,
migrations, tratamento de erros e gates automáticos de qualidade.

## Escopo

- **Dentro:** módulos por camada, configuração tipada, pool SQLx, migrations,
  Docker Compose de desenvolvimento, health checks, logs, CI e página mínima.
- **Fora:** autenticação e regras da carteira.

## Tarefas

- [x] Criar biblioteca + binário fino e módulos definidos no `SPEC.md`.
- [x] Fixar toolchain Rust e dependências; manter `Cargo.lock`.
- [x] Validar configuração no startup, com `.env.example` sem segredos.
- [x] Subir PostgreSQL 18.4 para desenvolvimento, sem publicar DB além de
  localhost; usar healthcheck e volume nomeado.
- [x] Criar migrations up/down para usuários, sessões e ativos, incluindo
  constraints e índices; validar ciclo completo em banco descartável.
- [x] Separar credenciais de migration e runtime com menor privilégio.
  - Dev usa `DATABASE_MIGRATION_URL` para SQLx migrations e `DATABASE_URL` para
    a aplicação. O usuário runtime não deve ter privilégio de DDL.
- [x] Implementar erro interno tipado e resposta pública estável com
  `correlation_id`; eliminar panics de caminhos normais.
- [x] Configurar tracing estruturado com redação de campos sensíveis.
- [x] Implementar `/health/live` e `/health/ready` sem revelar configuração.
- [x] CI: `fmt --check`, Clippy `-D warnings`, testes, migration check,
  `cargo audit`, `cargo deny`, auditoria npm e detecção de segredos.

## Critérios de aceitação

- [x] Ambiente limpo sobe com instruções documentadas.
- [x] Migrations aplicam, revertem e reaplicam sem erro.
- [x] Aplicação recusa configuração inválida e não imprime segredo.
- [x] Readiness falha quando o banco está indisponível; liveness continua útil.
- [x] Pipeline inteiro está definido no repositório e pronto para execução no
  GitHub Actions.

## Pendências de validação

- Credenciais reais de produção e rotação permanecem no repositório privado de
  infraestrutura; este Git contém somente nomes/segredos de desenvolvimento.
- Decidir se a aplicação deve iniciar sem conexão inicial ao banco para manter
  `/health/live` disponível quando o PostgreSQL estiver fora, ou se falha de
  startup é a política desejada para produção.
- A execução remota do GitHub Actions deve ser confirmada depois que o
  repositório público for criado e o primeiro push for feito.

## Validação executada em 2026-08-01

- CI público definido em `.github/workflows/ci.yml`.
- Job principal usa PostgreSQL via `docker-compose.dev.yml`, aplica migrations
  com `DATABASE_MIGRATION_URL` e executa a aplicação/testes com
  `DATABASE_URL` runtime.
- CI inclui TypeScript, build de assets, `npm audit`, gate público contra
  vazamento de infraestrutura, `cargo fmt`, Clippy e testes.
- Job de supply-chain Rust executa `cargo audit` e `cargo deny check`.

## Validação executada em 2026-07-28

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- execução sem `DATABASE_URL`, retornando erro de configuração obrigatório sem
  imprimir segredo
- `sqlx migrate run`
- `sqlx migrate revert --target-version 0`
- `sqlx migrate run`
- `GET /health/live` respondeu `{"status":"live"}`
- `GET /health/ready` respondeu `{"status":"ready"}`

## Validação executada em 2026-07-29

- Testes HTTP do router validam `correlation_id` no envelope público de erro e
  header `x-request-id` correspondente.
- Teste HTTP valida `/health/ready` retornando `503` controlado quando o banco
  está indisponível.

## Riscos e mitigação

- **Risco:** `.env` ou token versionado. → **Mitigação:** ignore, exemplo vazio,
  scanner de segredo e revisão da CI.
- **Risco:** readiness causar carga no DB. → **Mitigação:** consulta mínima,
  timeout curto e frequência controlada externamente.
