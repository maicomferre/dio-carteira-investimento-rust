# Carteira de Investimentos — Especificação

> Fonte de verdade dos requisitos atuais. Decisões, tarefas e validações de cada
> etapa ficam em [`specs/`](specs/README.md). Alterações de escopo devem atualizar
> este documento e o plano afetado antes da implementação.

- **Status:** levantamento inicial
- **Criado em:** 2026-07-25
- **Plataforma:** servidor Linux; navegador moderno no cliente
- **Prioridade:** segurança > correção > estabilidade > desempenho > conveniência

## 1. Objetivo e escopo

Construir uma aplicação web full-stack em Rust para cada pessoa cadastrar,
consultar e atualizar os próprios investimentos e visualizar o valor total da
carteira. O MVP demonstra, obrigatoriamente, Axum, PostgreSQL com SQLx,
autenticação com JWT em cookie e páginas Askama.

Melhoria autoral sobre a base DIO: corretoras, ativos e movimentações pertencem
a uma pessoa. Cada movimentação registra em qual corretora ocorreu. Cada ativo
possui símbolo, nome, categoria, moeda e preço atual; compras e vendas formam um
extrato imutável. Posição e preço médio são derivados por corretora e ativo. O
dashboard calcula `valor_atual = quantidade × preço_atual` e soma valores
somente dentro da mesma moeda. Valores monetários nunca usam `f32`/`f64`.

Ficam fora do MVP: atualização automática/contínua de cotações, envio ou
execução de ordens em corretora, custódia, recomendação financeira, conversão
cambial, recuperação de senha por e-mail, MFA, aplicativo móvel e microserviços.
Consulta pontual de metadados e preço indicativo para auxiliar o cadastro está
dentro do escopo.

## 2. Requisitos funcionais

- **RF-01:** cadastrar usuário em fluxo separado, com nome de usuário único e
  senha forte.
- **RF-02:** autenticar, renovar a sessão quando aplicável e encerrar a sessão.
- **RF-03:** criar, listar, detalhar e atualizar ativos do usuário autenticado.
- **RF-04:** impedir leitura ou alteração de ativos de outro usuário, inclusive
  por acesso direto a identificadores.
- **RF-05:** exibir dashboard responsivo com posições e totais por moeda.
- **RF-06:** registrar compras e vendas e exibir extrato cronológico, sem
  permitir venda superior à posição disponível.
- **RF-07:** exibir distribuição por categoria, participação por ativo e fluxo
  diário de compras/vendas quando existirem dados suficientes.
- **RF-08:** cadastrar, editar e arquivar corretoras; toda movimentação deve
  referenciar uma corretora ativa do próprio usuário.
- **RF-09:** exibir distribuição patrimonial por corretora somente quando ao
  menos duas corretoras tiverem posição positiva na moeda selecionada.
- **RF-10:** sugerir metadados ao buscar um símbolo, usando proxy e cache no
  backend, confirmação humana e cadastro manual como fallback.
- **RF-11:** validar formulários no navegador para UX e repetir toda validação no
  servidor, que é a autoridade.
- **RF-12:** oferecer respostas HTML e endpoints JSON sob `/api`; erros não
  devem expor detalhes internos.
- **RF-13:** aplicar e reverter o schema exclusivamente por migrations SQLx.

## 3. Modelo de dados inicial

- `users`: UUID, username normalizado, hash de senha, timestamps e estado.
- `auth_sessions`: UUID, usuário, hash/identificador da sessão JWT, expiração,
  revogação e metadados mínimos de auditoria.
- `brokers`: UUID, `user_id`, nome, nome normalizado, estado ativo/arquivado,
  observação opcional e timestamps.
- `assets`: UUID, `user_id`, símbolo, nome, mercado, categoria, moeda ISO 4217,
  preço atual `NUMERIC(20,8)`, identificador externo opcional, fonte e instante
  dos metadados/cotação, versão e timestamps.
- `transactions`: UUID, `user_id`, `broker_id`, `asset_id`, tipo `BUY|SELL`,
  data efetiva, quantidade `NUMERIC(28,10)`, preço unitário `NUMERIC(20,8)`,
  taxas `NUMERIC(20,8)`, observação opcional e timestamps.

No MVP, as moedas aceitas são BRL e USD, com BRL como padrão. Ativos negociados
na B3, incluindo BDRs comprados em reais, usam BRL; instrumentos efetivamente
negociados em mercado americano podem usar USD. Não haverá conversão automática
nem total único entre moedas. Uma futura consolidação exigirá fonte, instante e
histórico da taxa de câmbio.

Categorias iniciais: ação, FII, ETF, BDR, cripto e outros. Renda fixa fica fora
do primeiro MVP porque marcação, rendimento e resgate não seguem necessariamente
o mesmo modelo simples de quantidade × preço. O mercado (`B3`, `NASDAQ`, `NYSE`,
`CRYPTO` ou `OTHER`) elimina ambiguidades de símbolo e orienta a moeda. Essas
regras permanecem válidas mesmo quando o provider externo está indisponível.

O fluxo líquido diário é um fluxo de caixa, não lucro:
`vendas − compras − taxas`. Compra é saída; venda é entrada. Rentabilidade e
ganho realizado exigem regras próprias e não serão inferidos desse gráfico.

A posição disponível para venda é calculada dentro da corretora selecionada, não
apenas no total do ativo. Corretora com movimentações não pode ser apagada e só
pode ser arquivada quando todas as posições nela forem zero; o histórico
permanece. Transferência de custódia entre corretoras será uma evolução própria
e nunca será representada falsamente como compra/venda.

Metadados externos são conveniência, não fonte de verdade automática. A pesquisa
passa pelo backend, que usa providers substituíveis, allowlist de hosts, timeout,
limite de resposta, validação de schema, rate limit, circuit breaker e cache. O
usuário precisa selecionar/confirmar a sugestão. Falha externa não impede o
cadastro manual. Atualização externa nunca sobrescreve silenciosamente uma
correção manual. Tokens de provider nunca chegam ao navegador.

Integridade é dupla: tipos/validação no domínio e `NOT NULL`, `CHECK`, `UNIQUE`
e chaves estrangeiras no PostgreSQL. Toda consulta usa parâmetros do SQLx.

## 4. Arquitetura e organização

Será um monólito modular, evitando a complexidade prematura de microserviços ou
workspace com muitos crates:

```text
src/
  domain/          # entidades, value objects e cálculos puros
  application/     # casos de uso e ports (traits)
  infrastructure/  # SQLx, JWT, configuração e adapters
  presentation/    # Axum, middleware, DTOs e views
templates/         # páginas e parciais Askama
frontend/          # TypeScript e estilos-fonte
static/dist/       # assets gerados e servidos localmente
migrations/        # migrations SQLx reversíveis
container/          # Dockerfile e build/scan genéricos; nunca scripts do servidor
tests/             # integração HTTP/DB e segurança
```

Dependências apontam para dentro: apresentação e infraestrutura dependem de
aplicação/domínio; domínio não conhece Axum, SQLx, Askama ou rede. Traits serão
criados apenas em fronteiras que precisem de substituição em testes. Arquivos
agrupam responsabilidades coesas; não haverá “um arquivo por função”.

## 5. Frontend

Askama renderiza HTML no servidor. Bootstrap estável e Bootstrap Icons serão
instalados por npm, versionados no lockfile e servidos localmente; a versão
verificada no levantamento é Bootstrap 5.3.8. TypeScript fornece apenas
melhorias progressivas e validação amigável, sem React ou SPA. A aplicação deve
continuar utilizável nas operações essenciais sem JavaScript.

Templates escapam conteúdo por padrão. São proibidos HTML não escapado, scripts
inline e dependências por CDN no modo de produção.

O frontend terá um cliente HTTP tipado e centralizado, responsável por
serialização, timeout/cancelamento, CSRF, `Accept`, tratamento consistente de
401/403/422/429/5xx e propagação do identificador de correlação. Ele não contém
regras de negócio nem transforma todo erro em modal. SweetAlert2, instalado via
npm e servido localmente, será usado para confirmações e feedback global;
validações de campo permanecem inline e acessíveis. `window.alert`,
`window.confirm`, URLs soltas e chamadas `fetch` espalhadas são proibidos.

O dashboard usa um avatar visual com as iniciais do usuário, sem upload de
imagem. Terá gráfico de rosca por categoria, barras horizontais por ativo e
gráficos de distribuição por corretora e de compras, vendas e fluxo líquido
diário. O gráfico por corretora só aparece com posição positiva em duas ou mais
corretoras. Cada visualização é filtrada por moeda e só aparece quando houver
dados; tabelas e resumos textuais continuam disponíveis para acessibilidade. A
biblioteca de gráficos será instalada via npm e servida localmente.

## 6. Segurança

Meta: controles aplicáveis do **OWASP ASVS 5.0 nível 2** e cobertura explícita
do **OWASP Top 10:2025**.

- Senhas com Argon2id e parâmetros revisáveis; nunca logar senha, token, cookie
  ou segredo.
- JWT curto em cookie `HttpOnly`, `Secure`, `SameSite=Lax`, `Path=/`; claims
  incluem `sub`, `sid/jti`, `iat`, `exp`, `iss` e `aud`. Chaves vêm de secret
  externo e suportam rotação. Logout revoga a sessão no servidor.
- CSRF token em formulários e header nas mutações JSON, mais verificação de
  `Origin`/`Referer`. CORS fechado por padrão.
- Autorização por proprietário em todo caso de uso e consulta; respostas de
  login são genéricas contra enumeração.
- Limites de corpo, timeouts, pool de DB limitado, rate limit global e mais
  restritivo em autenticação, limite de concorrência e backoff de login.
- CSP restrita, HSTS no HTTPS, `X-Content-Type-Options`, política de referrer,
  frame protection e cache desabilitado em páginas autenticadas.
- Erros falham de modo seguro; identificador de correlação é retornado e o
  detalhe fica apenas em log estruturado.
- Dependências travadas e auditadas (`cargo audit`, `cargo deny`, `npm audit`);
  imagens e ações de CI fixadas por versão/digest quando possível.
- Quando containers forem usados, cumprir o OWASP Docker Top 10: usuário
  não-root, patching, segmentação, defaults seguros, contexto mínimo, proteção
  de secrets/recursos, integridade da imagem, imutabilidade e logging.

O frontend reduz enganos e abuso acidental; nunca é considerado barreira de
segurança. Botnets e DDoS volumétrico não podem ser resolvidos somente pela
aplicação: a implantação deve minimizar portas, usar firewall, reverse proxy,
limites de conexão/requisição e permitir WAF/CDN quando exposta à Internet.

## 7. Rede, configuração e operação

Em desenvolvimento, Axum pode ouvir em `127.0.0.1:3000`. Em produção, permanece
atrás de reverse proxy com TLS; bind, portas e trusted proxies são configuração,
nunca constantes. O alvo preferencial usa containers isolados para app Rust e
PostgreSQL, sem exposição pública do banco. PostgreSQL 18.4 é a versão estável
alvo em 2026-07-25. Usuários de aplicação e migration terão privilégios
separados e mínimos.

Compartilhar o VPS atual é aceitável somente após medir picos de CPU, memória,
swap, disco e I/O e definir limites por container. Outro VPS é obrigatório se
não houver folga acordada ou se o risco dos sites atuais não puder compartilhar
o mesmo host: Compose reduz interferência e exposição, mas não isola kernel,
daemon Docker, firewall ou indisponibilidade física.

Configuração vem de ambiente/secret store, é validada no startup e possui
exemplo sem credenciais. O serviço terá `/health/live` e `/health/ready`,
shutdown gracioso, logs estruturados sem PII sensível, métricas essenciais,
backup e restauração testada.

O repositório será público. Nele podem ser versionados o Dockerfile, scripts
genéricos de build/scan da imagem, Compose exclusivamente de desenvolvimento e
exemplos de variáveis sem valores reais. Ficam proibidos no repositório público:
deploy remoto, Compose de produção, vhost/regras Nginx, Fail2ban, firewall,
systemd, backup, monitoramento, inventário, domínio/IP/SSH, usuários e paths do
servidor. Esses artefatos vivem somente num repositório privado de
infraestrutura ou secret store. Um gate de CI bloqueia `.env`, chaves, tokens e
configurações reais. O código da aplicação pode ser público sem depender de
segurança por obscuridade; a topologia operacional, entretanto, é informação
privada por decisão do proprietário.

## 8. Qualidade e testes

- Testes unitários cobrem domínio, validações e todos os cálculos, incluindo
  zero, escala decimal, arredondamento, overflow, múltiplas moedas, preço médio,
  posição, venda inválida e fluxo líquido.
- Testes de integração usam PostgreSQL real isolado e verificam migrations,
  constraints, repositories, rotas e transações.
- Testes de segurança cobrem autenticação, expiração/revogação, CSRF, IDOR,
  injeção, XSS, limites, headers e vazamento de erros.
- Meta inicial: 80% de linhas do domínio/aplicação, sem usar cobertura como
  substituto para os casos críticos. `fmt`, Clippy sem warnings, testes e
  auditorias bloqueiam a CI.
- `unwrap`, `expect` e `panic!` não são aceitos em caminhos normais de produção.

## 9. Estratégia para a base DIO

O projeto será implementado do zero neste repositório e comparado com a base
DIO como referência de aprendizado. Não será copiado código porque a base não
declara licença e contém escolhas didáticas incompatíveis com estes requisitos:
segredos hardcoded, `f64` monetário, ativos globais, cadastro implícito no login,
cookie sem todos os atributos, bind público fixo e controles de segurança
ausentes.

Não há conflito com o escopo do professor. Toda a stack e as operações exigidas
serão mantidas; a diferença é o endurecimento necessário para um projeto
orientado a produção. A única substituição visual é Tailwind por Bootstrap,
pois Tailwind aparece no exemplo, não como requisito do desafio.

## 10. Critério global de conclusão

O MVP só está pronto quando todos os critérios das fases estiverem atendidos,
uma instalação Linux limpa puder subir o sistema seguindo o README, migrations
up/down forem validadas, backup/restauração forem exercitados, o checklist ASVS
aplicável não tiver pendência crítica e nenhum usuário conseguir observar ou
alterar dados de outro.

## Referências normativas

- Base DIO: <https://github.com/digitalinnovationone/rust-fullstack-carteira-investimentos>
- OWASP ASVS 5.0: <https://owasp.org/www-project-application-security-verification-standard/>
- OWASP Top 10:2025: <https://owasp.org/Top10/2025/>
- OWASP Cheat Sheet Series: <https://cheatsheetseries.owasp.org/>
- PostgreSQL: <https://www.postgresql.org/>
- Bootstrap: <https://getbootstrap.com/>
- OWASP Docker Top 10: <https://owasp.org/www-project-docker-top-10/>
