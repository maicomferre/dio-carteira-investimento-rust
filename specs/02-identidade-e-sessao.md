# Fase 2 — Identidade, sessão e autorização

- **Status:** concluída
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-28

## Objetivo

Cadastrar e autenticar pessoas sem expor credenciais, permitir logout efetivo e
estabelecer a fronteira de autorização usada pelo restante do produto.

## Escopo

- **Dentro:** registro explícito, login, logout, JWT em cookie, sessão
  revogável, CSRF, rate limit e principal autenticado.
- **Fora:** OAuth, e-mail, recuperação de senha e MFA.

## Decisões

| Data | Decisão | Justificativa | Alternativas |
|---|---|---|---|
| 2026-07-25 | JWT curto referenciado por sessão persistida | Cumpre o desafio e permite logout/revogação | JWT totalmente stateless |
| 2026-07-25 | Registro separado do login | Evita criação acidental e enumeração confusa | Auto cadastro ao primeiro login |
| 2026-07-25 | Cookie, não localStorage | Reduz exposição do token a XSS | Bearer mantido no navegador |

## Tarefas

- [x] Normalizar username e definir limites de tamanho/caracteres.
- [x] Aplicar Argon2id com parâmetros registrados no hash PHC.
- [x] Emitir JWT com claims mínimas e validar algoritmo, assinatura, audiência,
  emissor, tempo e sessão não revogada.
- [x] Configurar cookie `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/` e
  expiração coerente; não aceitar token por query string.
- [x] Implementar CSRF token + validação de origem em toda mutação autenticada.
- [x] Revogar sessão no logout.
- [x] Expirar registros antigos com rotina segura.
- [x] Aplicar rate limit por IP e conta, atraso progressivo e resposta genérica
  no login; documentar uso correto de IP atrás de trusted proxy.
- [x] Rotacionar ID após autenticação e impedir session fixation.
- [x] Persistir apenas hash HMAC do `jti` da sessão, nunca o identificador puro.

## Critérios de aceitação

- [x] Login válido funciona e inválido não revela se a conta existe.
- [x] Token expirado, adulterado, com claim errada ou sessão revogada é rejeitado.
- [x] Logout invalida reutilização do token.
- [x] Requisição cross-site sem CSRF/origem válida é rejeitada.
- [x] Senha, JWT e cookie não aparecem em logs nem respostas de erro.

## Validação executada

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-features`
- Testes unitários automatizados:
  - JWT adulterado rejeitado.
  - JWT expirado rejeitado.
  - JWT com audiência errada rejeitado.
  - CSRF ausente/origem inválida rejeitados.
  - rate limit bloqueia após falhas configuradas.
- Smoke test local com PostgreSQL Docker:
  - `POST /auth/register` → `201`
  - `POST /auth/login` → `200`
  - `GET /auth/me` autenticado → `200`
  - `POST /auth/logout` sem CSRF → `403`
  - `POST /auth/logout` com CSRF → `204`
  - reutilização do cookie após logout → `401`

## Pendências técnicas

- A limpeza de sessões expiradas roda na inicialização da aplicação e remove
  sessões cujo `expires_at` é anterior à janela configurada por
  `AUTH_EXPIRED_SESSION_RETENTION_DAYS`.
- CSRF atual usa double-submit cookie + header; quando as telas Askama forem
  criadas, os formulários devem renderizar/enviar o mesmo token.
- Rate limit atual usa IP real da conexão TCP. Só aceitar `X-Forwarded-For` se
  houver camada explícita de trusted proxy configurada fora deste repositório.

## Riscos e mitigação

- **Risco:** confiar em `X-Forwarded-For` enviado pelo atacante. →
  **Mitigação:** só aceitar headers do proxy explicitamente confiável.
- **Risco:** Argon2 facilitar DoS de login. → **Mitigação:** concorrência
  limitada, rate limit e parâmetros calibrados no servidor alvo.
