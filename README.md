# Carteira de Investimentos

Aplicação fullstack em Rust desenvolvida para o desafio DIO/Santander de
Carteira de Investimentos. O sistema permite criar conta, autenticar usuário,
cadastrar corretoras, cadastrar ativos, registrar compras e vendas e acompanhar
a carteira por moeda, categoria, ativo e corretora.

Este projeto foi desenvolvido como implementação própria a partir dos requisitos
do desafio. O repositório-base da DIO foi usado como referência didática, sem
copiar código sem licença declarada.

## O que o projeto faz

- Cadastro, login e logout com sessão em cookie `HttpOnly`.
- Controle CSRF em mutações autenticadas.
- Cadastro de corretoras usadas pelo investidor.
- Cadastro de ativos com símbolo, mercado, categoria e moeda.
- Sugestão de metadados de instrumentos pelo backend.
- Registro de compras e vendas com bloqueio de posição negativa.
- Dashboard com avatar por iniciais, totais por moeda, distribuição por
  categoria, participação por ativo, divisão por corretora e fluxo diário.
- Extrato de movimentações para rastrear compras e vendas.
- Health checks para execução em Linux atrás de um gateway HTTP.

O objetivo não é fazer recomendação financeira nem integração com corretoras
reais. O preço executado é informado em cada compra ou venda.

## Tecnologias usadas

- Rust 1.95.0, Axum e Tokio para a aplicação web.
- Askama para páginas HTML server-rendered.
- PostgreSQL 18.4 com SQLx e migrations versionadas.
- Argon2id para senha e JWT HS256 próprio para sessão.
- TypeScript, Bootstrap, Bootstrap Icons e SweetAlert2 no frontend.
- Docker Compose para banco local de desenvolvimento.
- GitHub Actions, `cargo audit`, `cargo-deny`, `npm audit` e Trivy para CI e
  auditoria pública.

## Arquitetura

O código segue separação por camadas:

```text
src/domain/          regras de negócio, tipos validados e cálculos
src/application/     casos de uso, orquestração e erros de aplicação
src/infrastructure/  PostgreSQL, configuração, segurança, telemetria e repos
src/presentation/    rotas HTTP, cookies, templates e boundary web/API
migrations/          schema PostgreSQL versionado pelo SQLx
frontend/            TypeScript do cliente progressivo
templates/           páginas Askama
specs/               requisitos, ameaças, rotas e planos por fase
```

Valores monetários e quantidades usam tipos decimais; o projeto não usa ponto
flutuante para cálculos financeiros.

## Como executar localmente

Pré-requisitos:

- Linux ou ambiente compatível.
- Rust 1.95.0.
- Node.js/npm.
- Docker com Compose.
- `sqlx-cli` 0.9.0.

Instale o `sqlx-cli` se necessário:

```bash
cargo install sqlx-cli --version 0.9.0 --no-default-features --features postgres,rustls
```

Configure o ambiente:

```bash
cp .env.example .env
```

Para desenvolvimento local, gere novos valores para `AUTH_JWT_SECRET` e
`AUTH_SESSION_HASH_KEY` no `.env`:

```bash
head /dev/urandom | tr -dc A-Za-z0-9 | head -c 64
```

Suba o banco e aplique migrations:

```bash
docker compose -f docker-compose.dev.yml up -d --wait
npm run db:migrate
```

Compile os assets e rode a aplicação:

```bash
npm install
npm run build
cargo run
```

Acesse `http://127.0.0.1:3000`.

O PostgreSQL de desenvolvimento usa `127.0.0.1:5433` para evitar conflito com
instalações locais na porta `5432`. A aplicação usa duas URLs: uma credencial de
migration (`DATABASE_MIGRATION_URL`) e uma de runtime (`DATABASE_URL`) com
privilégios menores.

## Rotas principais

- `GET /` — redireciona visitante para `/login` e usuário autenticado para
  `/dashboard`.
- `GET /login` e `GET /register` — páginas de autenticação.
- `POST /login`, `POST /register` e `POST /logout` — fallback HTML sem
  JavaScript.
- `GET /dashboard` — resumo autenticado da carteira.
- `GET /brokers`, `GET /assets`, `GET /transactions` — telas principais da
  carteira.
- `GET /health/live` — liveness sem banco.
- `GET /health/ready` — readiness com consulta ao PostgreSQL.
- `/auth/*` — API JSON de autenticação.
- `/api/brokers`, `/api/assets`, `/api/transactions`,
  `/api/portfolio/summary` — API JSON da carteira.

O contrato detalhado está em `specs/CONTRATO_HTTP.md` e o mapa funcional de
telas está em `specs/ROTAS_E_TELAS.md`.

## Como testar

Checks principais:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
npm run check
npm run test:ts
npm run build
npm run audit:public-boundary
```

Auditorias adicionais:

```bash
npm run audit:supply-chain
npm run audit:container
npm run audit:container-baseline
```

`cargo test --all-features` exige PostgreSQL local com migrations aplicadas. O
CI executa formatação, lint, testes Rust, testes TypeScript, auditoria pública
de fronteira e checks de supply chain.

Evidências públicas da entrega, exemplos de API sanitizados e roteiro de
screenshots ficam em `docs/ENTREGA_DIO_EVIDENCIAS.md`.

Screenshots já disponíveis:

- [Login](docs/screenshots/01-login.png)
- [Dashboard](docs/screenshots/02-dashboard.png)
- [Corretoras](docs/screenshots/03-corretoras.png)
- [Ativos](docs/screenshots/04-ativos.png)
- [Extrato](docs/screenshots/05-extrato.png)

## Melhoria autoral implementada

Além do fluxo básico proposto no desafio, esta versão adiciona:

- corretoras como entidade inicial da carteira;
- isolamento por usuário em consultas e mutações;
- controle de versão otimista para reduzir sobrescrita acidental;
- cálculo decimal seguro de posições, preço médio e totais por moeda;
- dashboard com gráficos que não misturam BRL e USD;
- extrato com compras, vendas e fluxo líquido diário;
- autenticação com cookie, CSRF, rate limit e eventos de segurança;
- separação entre usuário de migration e usuário runtime do banco;
- container runtime non-root e baseline público inspirado no OWASP Docker Top
  10.

## Segurança e limites de produção

Segurança é requisito primário do projeto. O backend não confia em entrada do
usuário, usa DTOs com campos conhecidos, queries parametrizadas via SQLx,
cookies `HttpOnly`, cabeçalhos de segurança, limite de corpo, timeout, limite de
concorrência e rate limits em memória.

Limitações honestas do MVP:

- rate limit em memória protege uma instância, mas mitigação forte de DDoS exige
  camada de borda privada;
- preços não vêm de uma integração financeira oficial em tempo real;
- não há recuperação de senha, upload de avatar ou painel administrativo;
- métricas, alertas, Nginx, Fail2ban, firewall, backup e deploy real ficam fora
  deste repositório público.

O contrato público para Linux/produção está em `specs/07-producao-linux.md`.
Detalhes reais de VPS, domínio, usuários, paths, chaves, regras de proxy e
runbooks operacionais não devem ser versionados aqui.

## O que foi aprendido no desafio

- Como estruturar uma aplicação fullstack Rust com Axum, Askama e SQLx.
- Como modelar regras financeiras sem `float`.
- Como separar domínio, aplicação, infraestrutura e apresentação sem exagerar a
  fragmentação de arquivos.
- Como usar migrations e credenciais distintas para evolução segura do banco.
- Como aplicar segurança em camadas: frontend para experiência, backend como
  autoridade, banco com invariantes e CI com auditorias.
- Como preparar um projeto pequeno para futura hospedagem Linux sem expor
  detalhes sensíveis de infraestrutura em um repositório público.

## Uso dos specs no desenvolvimento

O projeto foi evoluído a partir dos documentos em `specs/`, que funcionaram
como trilha de requisitos e controle de escopo. Primeiro foram definidos domínio,
ameaças, rotas, telas e decisões técnicas. Depois, cada fase guiou a
implementação: autenticação, carteira, cálculos, interface, hardening, Docker,
produção Linux e documentação da entrega.

Esse fluxo ajudou a evitar decisões soltas no código. Mudanças como corretoras,
extrato por movimentação, cálculo decimal, limites de entrada, separação de
credenciais do banco, auditoria pública e fronteira privada de VPS foram
registradas antes ou junto da implementação. Assim, o código atual reflete uma
evolução planejada, testável e alinhada aos requisitos do desafio.

## Licença

Este projeto está licenciado sob MIT. Veja `LICENSE`.
