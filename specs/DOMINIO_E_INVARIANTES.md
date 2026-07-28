# Dominio e invariantes

> Contrato de negocio para orientar migrations, casos de uso e testes. Nenhuma
> regra desta pagina deve depender de template, JavaScript ou detalhe de banco.

## Entidades

### User

- `username` e unico apos normalizacao: trim, lowercase e limite de tamanho.
- Senha nunca e armazenada em texto claro; apenas hash Argon2id.
- Usuario inativo nao autentica e nao pode criar novos registros.

### AuthSession

- Sessao pertence a exatamente um usuario.
- JWT e curto e referenciado por `sid`/`jti` revogavel no servidor.
- Logout revoga a sessao atual; sessoes expiradas nao renovam.

### Broker

- Corretora pertence a um usuario.
- Nome normalizado e unico por usuario enquanto ativa.
- Toda transacao referencia uma corretora ativa do proprio usuario.
- Corretora com historico nao e apagada fisicamente.
- Arquivamento so e permitido quando todas as posicoes nela forem zero.

### Asset

- Ativo pertence a um usuario.
- `symbol`, `market` e `currency` eliminam ambiguidades basicas.
- `current_price` e informativo para calculo de valor atual, nao preco de compra.
- Metadado externo nunca sobrescreve correcao manual sem confirmacao.
- Ativo nao armazena quantidade; quantidade deriva do extrato.

### Transaction

- Movimentacao pertence a usuario, ativo e corretora do mesmo proprietario.
- Tipos iniciais: `BUY` e `SELL`.
- Quantidade, preco unitario e taxas usam decimal; nunca `f32` ou `f64`.
- Venda nao pode exceder a posicao disponivel na corretora selecionada.
- Correcoes futuras devem usar cancelamento/substituicao auditavel.

## Value objects

| Objeto | Regra |
|---|---|
| `Currency` | MVP aceita `BRL` e `USD`; totais nunca misturam moedas. |
| `Market` | Allowlist: `B3`, `NASDAQ`, `NYSE`, `CRYPTO`, `OTHER`. |
| `AssetSymbol` | Uppercase, sem espacos internos, tamanho maximo definido no dominio. |
| `Money` | Decimal positivo ou zero conforme o campo; escala preservada no banco. |
| `Quantity` | Decimal estritamente positivo em transacoes. |
| `Fee` | Decimal maior ou igual a zero. |

## Calculos

- Valor atual: `quantity * current_price`.
- Compra aumenta quantidade e custo total pela soma de `quantity * unit_price + fees`.
- Venda reduz quantidade. No MVP, o grafico diario trata venda como entrada de
  caixa e nao calcula lucro realizado.
- Preco medio ponderado e recalculado a partir do historico valido.
- Fluxo liquido diario: `sales - purchases - fees`.
- Arredondamento ocorre apenas na apresentacao; persistencia e calculo mantem
  precisao decimal.

## Casos limite obrigatorios

- Compra inicial, compras sucessivas e venda parcial.
- Mesmo ativo em duas corretoras.
- Mesma sigla em mercados diferentes.
- Taxa zero e taxa positiva.
- Simbolo inexistente no provider com cadastro manual.
- Concorrencia entre duas vendas que poderiam negativar a posicao.
