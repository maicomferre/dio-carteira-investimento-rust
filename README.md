# Carteira de Investimentos

Aplicação fullstack em Rust para o desafio DIO/Santander. O objetivo do MVP é
permitir cadastro, autenticação e acompanhamento de ativos de investimento com
Axum, PostgreSQL, SQLx, JWT/cookies e páginas Askama.

## Estado atual

O projeto já possui base técnica, autenticação e núcleo JSON da carteira. Existem
estrutura por camadas, health checks, configuração por ambiente, migrations,
PostgreSQL de desenvolvimento via Docker, registro/login/logout e rotas para
corretoras, ativos, movimentações e resumo da carteira.

## Requisitos locais

- Rust 1.95.0
- Docker com Compose
- PostgreSQL 18.4 via `docker-compose.dev.yml`

## Execução local

```bash
cp .env.example .env
docker compose -f docker-compose.dev.yml up -d
sqlx migrate run
cargo run
```

O PostgreSQL de desenvolvimento é publicado em `127.0.0.1:5433` para evitar
conflito com instalações locais que já usam a porta padrão `5432`.

Se o `sqlx-cli` não estiver instalado:

```bash
cargo install sqlx-cli --version 0.9.0 --no-default-features --features postgres,rustls
```

Endpoints iniciais:

- `GET /` — página/texto mínimo da aplicação.
- `GET /health/live` — liveness sem dependência do banco.
- `GET /health/ready` — readiness com consulta mínima ao PostgreSQL.
- `POST /auth/register` — cadastra usuário com senha hasheada via Argon2id.
- `POST /auth/login` — autentica e grava JWT em cookie `HttpOnly`.
- `GET /auth/me` — retorna usuário autenticado via cookie.
- `POST /auth/logout` — exige `x-csrf-token`, revoga a sessão persistida e
  limpa os cookies.
- `GET/POST /api/brokers` — lista e cadastra corretoras.
- `PATCH /api/brokers/{broker_id}` — atualiza corretora com controle de versão.
- `POST /api/brokers/{broker_id}/archive` — arquiva corretora sem posição
  aberta.
- `GET/POST /api/assets` — lista e cadastra ativos.
- `PATCH /api/assets/{asset_id}` — atualiza ativo com allowlist e versão.
- `GET /api/instruments/search?q=PETR4` — sugere metadados de ativo pelo
  backend com cache; no MVP usa fonte local determinística e exige confirmação
  do usuário antes de cadastrar.
- `GET /api/transactions` — lista extrato.
- `POST /api/transactions/buy` — registra compra.
- `POST /api/transactions/sell` — registra venda, bloqueando posição negativa.
- `GET /api/portfolio/summary` — retorna posições, totais por moeda, alocações
  e fluxo diário.

Exemplo de login local:

```bash
curl -c /tmp/carteira-cookie.txt \
  -H 'content-type: application/json' \
  -d '{"username":"maicom_dev","password":"SenhaLocalForte123"}' \
  http://127.0.0.1:3000/auth/login
```

O login também retorna `csrf_token` e grava o cookie `investment_csrf`. Toda
mutação autenticada deve enviar esse valor no header `x-csrf-token`. O app ainda
aplica rate limit em memória por IP real da conexão e username normalizado; no
VPS, Nginx/Fail2ban continuam fora deste repositório público.

## Qualidade

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Configurações reais de VPS, Nginx, Fail2ban, firewall, deploy remoto, backups e
segredos não pertencem a este repositório público.

A aplicação carrega somente o arquivo `./.env` do diretório atual. Ela não deve
herdar `.env` de diretórios pais para evitar contaminação entre projetos locais.
