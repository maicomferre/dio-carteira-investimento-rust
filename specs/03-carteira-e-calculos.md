# Fase 3 — Carteira, ativos e cálculos

- **Status:** em execução
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-28

## Objetivo

Entregar o núcleo do desafio: administrar corretoras e ativos, registrar
compras/vendas e produzir posição, extrato e cálculos determinísticos, sempre
isolados por usuário e corretora.

## Escopo

- **Dentro:** domínio, corretoras, catálogo externo com fallback manual, casos
  de uso, repositories SQLx, API, ownership, extrato, posição, preço médio,
  totais e testes unitários/integrados.
- **Fora:** exclusão física, dividendos, desdobramentos, vendas a descoberto,
  atualização contínua de cotações, imposto e conversão cambial.

## Decisões

| Data | Decisão | Justificativa | Alternativas |
|---|---|---|---|
| 2026-07-25 | `rust_decimal` + PostgreSQL `NUMERIC` | Dinheiro não tolera erro binário de `f64` | Ponto flutuante |
| 2026-07-25 | Totais separados por moeda | Somar BRL e USD sem câmbio é incorreto | Total global enganoso |
| 2026-07-25 | Ownership compõe toda consulta | Evita IDOR por construção | Buscar por ID e checar depois |
| 2026-07-25 | Posição deriva do extrato | Evita divergência entre quantidade editável e operações | Armazenar quantidade manual |
| 2026-07-25 | Movimentação não é apagada | Preserva rastreabilidade; correção é cancelamento/substituição auditável | CRUD destrutivo |
| 2026-07-25 | Corretora obrigatória na movimentação | Permite custódia, taxas e análises coerentes desde o início | Campo opcional adicionado depois |
| 2026-07-25 | Provider externo atrás de trait/cache | Evita acoplamento e mantém testes/cadastro funcionando offline | API chamada diretamente no browser |
| 2026-07-28 | Compra/venda usa transação com lock por posição | Evita corrida entre duas vendas do mesmo ativo/corretora sem bloquear a carteira inteira | Validar fora da transação |
| 2026-07-28 | MVP usa provider local determinístico atrás do mesmo cache | Mantém cadastro prático e testável até escolher fonte externa por licença/limites | Acoplar já a uma API pública sem análise |

## Tarefas

- [x] Implementar value objects para símbolo, moeda, quantidade, preço, taxas e
  tipo de movimentação.
- [x] Definir arredondamento de exibição e preservar precisão no armazenamento.
- [x] Criar casos de uso `CreateAsset`, `ListAssets`, `GetAsset` e `UpdateAsset`.
- [x] Criar `CreateBroker`, `ListBrokers`, `UpdateBroker` e `ArchiveBroker`;
  permitir arquivamento somente com posições zeradas e nunca excluir histórico.
- [x] Criar `RecordPurchase`, `RecordSale` e `ListStatement`; bloquear posição
  negativa na corretora selecionada.
- [x] Tratar duas vendas concorrentes atomicamente em transação serializável ou
  mecanismo equivalente de lock por posição.
- [x] Definir `InstrumentProvider` e cache; provider real fica na infraestrutura
  e testes usam fake determinístico, sem rede.
- [x] Normalizar sugestões externas para símbolo, nome, mercado, categoria,
  moeda, identificador/fonte e instante; exigir confirmação do usuário.
- [ ] Implementar retry limitado com jitter, circuit breaker e limite de
  payload para provider externo real; timeout, limite de resultados, cache,
  stale-if-error e validação estrita já existem no provider local/cache.
- [x] Usar queries SQLx parametrizadas; nenhuma concatenação SQL.
- [x] Exigir `user_id` no predicado de leitura/alteração e retornar resposta
  indistinguível para ativo inexistente ou alheio.
- [x] Implementar `PATCH /api/assets/{id}` com allowlist de campos.
- [x] Calcular quantidade, custo médio ponderado, posição e totais no domínio,
  sem lógica monetária em template.
- [x] Calcular séries diárias: compras, vendas e
  `fluxo_liquido = vendas − compras − taxas`; nunca rotular como lucro.
- [x] Tratar concorrência de atualização de ativo/corretora com versão e
  conflito 409.

## Implementado nesta etapa

- API autenticada para corretoras: `GET/POST /api/brokers`,
  `PATCH /api/brokers/{broker_id}` e `POST /api/brokers/{broker_id}/archive`.
- API autenticada para ativos: `GET/POST /api/assets` e
  `PATCH /api/assets/{asset_id}`.
- API autenticada para busca de metadados: `GET /api/instruments/search?q=PETR4`.
  A chamada passa pelo backend, usa timeout/cache/stale e nunca expõe segredo ou
  host de provider para o navegador.
- API autenticada para extrato e movimentações:
  `GET /api/transactions`, `POST /api/transactions/buy` e
  `POST /api/transactions/sell`.
- API de resumo: `GET /api/portfolio/summary`, com posições, totais por moeda,
  alocação por categoria, alocação por corretora e fluxo diário.
- Mutação autenticada exige cookie de sessão e header `x-csrf-token`.
- Compra/venda roda dentro de transação PostgreSQL com `pg_advisory_xact_lock`
  por `user_id + asset_id + broker_id`; venda revalida o extrato dentro do lock
  antes de inserir a movimentação.
- Smoke test local validou registro, login, criação de corretora, criação de
  ativo, compra, venda e resumo contra PostgreSQL dev.

## Casos de teste obrigatórios

- zero, valores mínimos/máximos e muitas casas decimais;
- arredondamento apenas na apresentação;
- ativo duplicado conforme regra definida;
- payload ausente, excessivo, negativo, NaN textual e campos desconhecidos;
- UUID inválido e ativo de outro usuário;
- duas atualizações concorrentes;
- compra inicial, compras sucessivas e venda parcial;
- mesmo ativo em duas corretoras, agregação e segregação corretas;
- tentativa de venda acima da posição naquela corretora e vendas concorrentes;
- corretora alheia, arquivada, duplicada ou com histórico;
- taxas em compra/venda e fluxo líquido diário;
- provider com sucesso, ambiguidade, símbolo inexistente, timeout, 429, resposta
  inválida, cache hit, stale fallback e modo manual;
- ordem por data efetiva e desempate estável;
- rollback de transação após falha;
- totais com vários ativos e moedas.

## Critérios de aceitação

- [x] Operações exigidas pelo professor funcionam por API e caso de uso.
- [x] Nenhuma consulta atravessa a fronteira do proprietário.
- [ ] Resultado decimal é idêntico entre domínio e banco.
- [x] Extrato reconstrói exatamente posição e custo médio.
- [x] Totais por corretora fecham com a soma de suas posições e respeitam moeda.
- [x] Constraints rejeitam estado inválido mesmo fora da aplicação.

## Riscos e mitigação

- **Risco:** precisão/escala incompatível entre Rust e PostgreSQL. →
  **Mitigação:** spike na Fase 0 e testes round-trip nos limites.
