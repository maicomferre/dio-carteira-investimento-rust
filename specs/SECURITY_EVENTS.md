# Eventos de Segurança e Operação

Última revisão: 2026-07-29.

Este documento define apenas os eventos emitidos pela aplicação. Regras reais de
alerta, thresholds, retenção, dashboards, Nginx, Fail2ban, firewall e automações
do VPS pertencem à infraestrutura privada e não são versionados aqui.

## Campos comuns

- `event`: nome estável do evento.
- `client_ip`: IP visto pela aplicação. Atrás de proxy, a configuração de IP
  real/trusted proxy é responsabilidade privada.
- `path`: rota pública chamada, quando disponível.
- `correlation_id`: identificador opaco devolvido ao usuário em erro `5xx`.
- `status` e `code`: status HTTP e código público do erro.

Não registrar senha, token, cookie, secret, payload completo, query string
sensível nem username em texto claro. Quando for necessário correlacionar
tentativas de login, usar `username_fingerprint`.

## Eventos emitidos

| Evento | Nível | Quando ocorre | Campos principais | Uso operacional esperado |
|---|---|---|---|---|
| `auth.login_failed` | `warn` | Credencial inválida após validação de rate limit | `client_ip`, `username_fingerprint` | Detectar força bruta ou enumeração |
| `auth.login_rate_limited` | `warn` | Login bloqueado por excesso de falhas | `client_ip`, `username_fingerprint` | Acionar proteção adicional privada |
| `http.rate_limited` | `warn` | Rate limit global, registro ou mutação bloqueia request | `client_ip`, `scope`, `path` | Detectar abuso por endpoint/escopo |
| `http.concurrency_saturated` | `warn` | Sem permissão disponível no limite interno de concorrência | `client_ip`, `path` | Detectar saturação do app antes de queda |
| `http.server_error` | `error` | Resposta pública `5xx` emitida por `AppError` | `status`, `code`, `correlation_id` | Correlacionar incidente com relato do usuário |
| `db.readiness_failed` | `error` | `/health/ready` não consegue consultar o PostgreSQL | `error` sanitizado pelo driver | Detectar indisponibilidade de dependência |

## Critério de privacidade

Eventos públicos devem ser úteis para correlação, mas insuficientes para revelar
topologia, segredo, regra de defesa ou dado financeiro. A camada privada decide
quais combinações de evento, IP, janela e volume acionam alerta ou bloqueio.
