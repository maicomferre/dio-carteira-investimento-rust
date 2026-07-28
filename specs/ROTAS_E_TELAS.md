# Rotas e telas do MVP

> Contrato funcional inicial. Paths podem mudar na Fase 0, mas qualquer mudança
> deve preservar os fluxos e controles descritos aqui.

## Fluxo principal

`GET /` funciona como porta de entrada: visitante segue para `/login`; pessoa
autenticada segue para `/dashboard`. Após login bem-sucedido, o destino padrão é
sempre `/dashboard`, salvo retorno seguro para uma rota interna previamente
solicitada. Redirect externo fornecido pelo usuário é proibido.

No dashboard aparecem:

- barra superior com marca, círculo com iniciais do usuário e logout;
- cartões com valor atual total, separados por moeda;
- tabela/lista dos ativos do próprio usuário;
- símbolo, mercado, nome, categoria, quantidade, preço médio, preço atual e
  valor atual;
- seletor de moeda BRL/USD;
- gráfico de rosca com distribuição por categoria;
- barras horizontais com participação de cada ativo;
- gráfico de rosca por corretora, somente quando duas ou mais tiverem posição
  positiva na moeda selecionada, calculado pelo valor atual custodiado;
- série diária de compras, vendas e fluxo líquido quando houver extrato;
- ações “Adicionar ativo”, “Registrar movimentação”, “Ver extrato”,
  “Corretoras” e “Editar”;
- estado vazio orientando o primeiro cadastro;
- feedback inline e por toast, sem dados de outros usuários.

Os gráficos nunca misturam moedas. Sem dados suficientes, são substituídos por
um estado vazio; não exibem zeros artificiais. O gráfico diário representa fluxo
de caixa, não lucro ou rentabilidade.

## Rotas HTML

| Método | Rota | Acesso | Resultado |
|---|---|---|---|
| GET | `/` | público | Redireciona para login ou dashboard |
| GET | `/register` | visitante | Formulário de cadastro |
| POST | `/register` | visitante + CSRF | Cria conta e inicia sessão |
| GET | `/login` | visitante | Formulário de login |
| POST | `/login` | visitante + CSRF + rate limit | Autentica e redireciona |
| POST | `/logout` | autenticado + CSRF | Revoga sessão e volta ao login |
| GET | `/dashboard` | autenticado | Resumo e ativos do usuário |
| GET | `/assets/new` | autenticado | Formulário de novo ativo |
| POST | `/assets` | autenticado + CSRF | Cria ativo |
| GET | `/assets/{id}/edit` | proprietário | Formulário de edição |
| POST | `/assets/{id}` | proprietário + CSRF | Atualiza via formulário HTML |
| GET | `/brokers` | autenticado | Lista corretoras do usuário |
| GET | `/brokers/new` | autenticado | Formulário de corretora |
| POST | `/brokers` | autenticado + CSRF | Cadastra corretora |
| GET | `/brokers/{id}/edit` | proprietário | Edita corretora |
| POST | `/brokers/{id}` | proprietário + CSRF | Atualiza/arquiva corretora |
| GET | `/transactions` | autenticado | Extrato com filtros |
| GET | `/transactions/new` | autenticado | Formulário de compra/venda |
| POST | `/transactions` | autenticado + CSRF | Registra movimentação |

“Corretoras” é uma área de domínio, não uma tela genérica de configurações. Não
haverá painel administrativo, perfil, configurações gerais, upload de avatar,
recuperação de senha ou exclusão de ativo no primeiro MVP. O rodapé será mínimo:
nome, versão, link do repositório e aviso de que não há recomendação financeira.
Erros 403/404/429/500 possuem páginas próprias e não expõem detalhes internos.

No cadastro de ativo, o usuário digita o símbolo e recebe sugestões do backend.
Ao escolher uma, nome, mercado, categoria, moeda e preço atual indicativo são
preenchidos com fonte e horário. Tudo pode ser revisado; indisponibilidade do
provider revela o formulário manual, sem bloquear o fluxo.

## API JSON

Todas as rotas usam `/api/v1`, exigem JSON e retornam envelope de erro estável
com correlation ID.

| Método | Rota | Finalidade |
|---|---|---|
| GET | `/api/v1/assets` | Listar ativos do usuário |
| POST | `/api/v1/assets` | Criar ativo |
| GET | `/api/v1/assets/{id}` | Detalhar ativo próprio |
| PATCH | `/api/v1/assets/{id}` | Atualizar campos permitidos |
| GET | `/api/v1/instruments/search?q=PETR4` | Buscar metadados via cache/provider |
| GET | `/api/v1/brokers` | Listar corretoras próprias |
| POST | `/api/v1/brokers` | Cadastrar corretora |
| PATCH | `/api/v1/brokers/{id}` | Editar ou arquivar corretora |
| GET | `/api/v1/portfolio/summary` | Totais calculados por moeda |
| GET | `/api/v1/portfolio/charts?currency=BRL` | Séries agregadas para gráficos |
| GET | `/api/v1/transactions` | Extrato filtrado do usuário |
| POST | `/api/v1/transactions` | Registrar compra ou venda |
| POST | `/api/v1/auth/refresh` | Renovar sessão elegível |

O cliente TypeScript central adiciona CSRF e headers, aplica timeout/cancelamento
e traduz 401/403/422/429/5xx. SweetAlert2 mostra confirmações e feedback global;
erros de validação são exibidos junto aos campos.

## Rotas operacionais

| Método | Rota | Exposição |
|---|---|---|
| GET | `/health/live` | Sonda local, sem detalhes |
| GET | `/health/ready` | Somente proxy/orquestrador |

Métricas, se adicionadas, não serão públicas; ficarão em porta/rede administrativa
ou exigirão autenticação de infraestrutura.
