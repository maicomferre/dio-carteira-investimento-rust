# Contrato HTTP e API

> Contrato inicial da Fase 0. O backend e a autoridade; validacao no frontend
> existe para experiencia do usuario, nao para seguranca.

## Convencoes

- HTML usa formularios com CSRF e redirecionamento pos-acao.
- JSON do MVP usa `/api`, `Content-Type: application/json`, `Accept:
  application/json` e header CSRF em mutacoes autenticadas.
- Todo erro JSON retorna `correlation_id`, `code` e `message` publica.
- Erros de validacao retornam campos individualizados.
- Recursos alheios e inexistentes devem ser indistinguiveis quando isso reduzir
  risco de enumeracao.

## Status padrao

| Status | Uso |
|---|---|
| `200` | Consulta ou atualizacao concluida com corpo. |
| `201` | Criacao concluida. |
| `204` | Acao sem corpo, quando aplicavel. |
| `303` | Redirecionamento apos POST HTML. |
| `400` | Requisicao malformada. |
| `401` | Sessao ausente, expirada ou invalida. |
| `403` | CSRF/origem invalida ou acao autenticada proibida. |
| `404` | Recurso inexistente ou fora do proprietario. |
| `409` | Conflito de versao, concorrencia ou regra de estado. |
| `422` | Payload valido como JSON/form, mas viola regra de campo. |
| `429` | Limite de requisicoes. |
| `500` | Falha interna sem detalhe sensivel. |
| `503` | Dependencia indisponivel em readiness ou provider externo. |

## Envelope de erro

```json
{
  "error": {
    "code": "validation_failed",
    "message": "Revise os campos informados.",
    "correlation_id": "01J...",
    "fields": {
      "symbol": ["Informe um simbolo valido."]
    }
  }
}
```

## Payloads principais

## Rotas implementadas no MVP

### Autenticação

- `POST /auth/register`
- `POST /auth/login`
- `GET /auth/me`
- `POST /auth/logout`

### Carteira

- `GET /api/brokers`
- `POST /api/brokers`
- `PATCH /api/brokers/{broker_id}`
- `POST /api/brokers/{broker_id}/archive`
- `GET /api/assets`
- `POST /api/assets`
- `PATCH /api/assets/{asset_id}`
- `GET /api/instruments/search?q=PETR4`
- `GET /api/transactions`
- `POST /api/transactions/buy`
- `POST /api/transactions/sell`
- `GET /api/portfolio/summary`

Mutações autenticadas exigem cookie `investment_session`, cookie
`investment_csrf` e header `x-csrf-token` com o mesmo valor do cookie CSRF.

### Criar ativo

```json
{
  "symbol": "PETR4",
  "name": "Petrobras PN",
  "market": "B3",
  "category": "STOCK",
  "currency": "BRL",
  "current_price": "38.42"
}
```

### Buscar instrumento

`GET /api/instruments/search?q=PETR4`

```json
{
  "items": [
    {
      "symbol": "PETR4",
      "name": "Petrobras PN",
      "market": "B3",
      "category": "stock",
      "currency": "BRL",
      "indicative_price": "38.42",
      "source": "local-fixture",
      "source_instrument_id": "PETR4",
      "as_of_unix": 1785240000
    }
  ],
  "cache": "fresh"
}
```

### Criar corretora

```json
{
  "name": "Nubank"
}
```

### Registrar movimentacao

```json
{
  "asset_id": "uuid",
  "broker_id": "uuid",
  "quantity": "100",
  "unit_price": "38.42",
  "fees": "1.50",
  "occurred_at_unix": 1785265200,
  "notes": "Compra teorica"
}
```

## Regras por rota critica

- `POST /auth/login`: resposta generica para credencial invalida; rate limit mais
  restritivo; sucesso cria cookie seguro.
- `POST /auth/logout`: idempotente para UX, mas registra tentativa com sessao valida.
- `PATCH /api/assets/{id}`: allowlist de campos; exige versao ou
  `updated_at` para detectar concorrencia.
- `POST /api/transactions/sell`: bloqueia venda que deixaria posicao negativa e
  revalida dentro de transacao com lock PostgreSQL por posicao.
- `GET /api/portfolio/summary`: retorna dados para totais, rosca por categoria,
  barras por ativo/corretora e fluxo diario.
