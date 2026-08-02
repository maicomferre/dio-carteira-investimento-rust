# Fase 4 — Interface web server-rendered

- **Status:** em andamento
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-08-02

## Objetivo

Oferecer uma interface responsiva e acessível com Askama, Bootstrap e
TypeScript, mantendo HTML server-rendered e complexidade baixa.

## Escopo

- **Dentro:** layouts, cadastro/login, dashboard, formulários de ativo,
  mensagens de erro, acessibilidade e melhorias progressivas.
- **Fora:** SPA, React, estado global no navegador e gráficos de rentabilidade
  histórica sem dados que os sustentem.

## Tarefas

- [x] Instalar Bootstrap e Bootstrap Icons pelo npm com versões fixadas e assets
  locais; não usar CDN em produção.
- [x] Instalar SweetAlert2 pelo npm e encapsular confirmações, toasts e mensagens
  globais num adapter tipado; não usar `window.alert`/`window.confirm`.
- [x] Compilar TypeScript estrito para `static/dist`; sem JavaScript inline.
- [x] Criar `HttpClient`/orquestrador único com generics para request/response,
  `AbortController`, timeout, CSRF, headers e tratamento uniforme de erros.
- [x] Mapear 422 para erro de validação, 401 para retorno ao login, 403 para
  acesso negado, 429 para espera orientada e 5xx para mensagem com correlation ID.
- [ ] Centralizar rotas em configuração gerada pelo servidor; proibir URLs
  hardcoded e chamadas `fetch` diretas fora do cliente HTTP. Rotas TypeScript já
  estão agrupadas, mas ainda não são geradas pelo servidor.
- [x] Criar layout Askama, parciais e páginas com escaping padrão.
- [x] Exibir no header um círculo com iniciais derivadas do username, sem upload
  ou URL de avatar.
- [x] Implementar formulários acessíveis, labels, foco de erro, navegação por
  teclado e contraste; idioma padrão `pt-BR`.
- [x] Validar no cliente limites e formato para feedback imediato, mantendo a
  mesma validação autoritativa no servidor.
- [x] Exibir dashboard com posições e totais por moeda.
- [x] Adicionar seletor BRL/USD e ocultar qualquer tentativa de somar moedas.
- [x] Renderizar gráfico de rosca por categoria e barras horizontais por ativo,
  ambos referentes à moeda selecionada.
- [x] Renderizar rosca de distribuição por corretora apenas se duas ou mais
  corretoras tiverem posição positiva na moeda selecionada; percentuais usam o
  valor atual das posições, não aportes históricos.
- [x] Renderizar série diária com compras, vendas e fluxo líquido somente quando
  houver movimentações; mostrar estado explicativo quando não houver dados.
- [x] Fornecer tabela/resumo textual equivalente aos gráficos para
  acessibilidade e fallback visual.
- [x] Criar tela de extrato e registro de compra/venda com seleção de ativo e
  corretora.
- [x] Adicionar filtros do extrato por período, ativo e tipo.
- [x] Criar tela “Corretoras” para cadastrar, editar e arquivar; não usar uma
  tela genérica de configurações no MVP.
- [x] No cadastro do ativo, buscar símbolo com debounce no backend, permitir
  seleção inequívoca e manter modo manual.
- [x] Adicionar cancelamento da busca de ativo e exibir instante da fonte.
- [x] No registro da movimentação, exigir corretora.
- [x] No registro da movimentação, mostrar a posição
  disponível daquele ativo dentro dela.
- [x] Preservar valores seguros após erro e nunca preencher novamente senha.
- [x] Definir CSP compatível com assets locais e eliminar `unsafe-inline`.
- [ ] Verificar viewport móvel e navegadores modernos.

## Critérios de aceitação

- [ ] Fluxos essenciais funcionam sem JavaScript.
- [ ] Conteúdo fornecido por usuário não executa HTML/JavaScript.
- [ ] Não há recurso remoto obrigatório nem erro no console.
- [ ] Testes TypeScript cobrem sucesso, timeout, cancelamento e cada classe de
  resposta do cliente HTTP, além do adapter SweetAlert2.
- [x] Formulários são utilizáveis por teclado e comunicam erros a leitores de tela.
- [ ] Gráficos não misturam moedas e não chamam fluxo líquido de lucro.
- [ ] Gráfico por corretora fica ausente com zero ou uma instituição investida.
- [ ] Bootstrap substitui integralmente o Tailwind da referência.

## Riscos e mitigação

- **Risco:** lógica de negócio migrar para TypeScript/template. →
  **Mitigação:** frontend só apresenta; casos de uso e cálculos permanecem Rust.
- **Risco:** dependência npm comprometer supply chain. → **Mitigação:** lockfile,
  auditoria, assets locais e revisão de atualização.
- **Risco:** um orquestrador HTTP virar framework interno. → **Mitigação:** API
  pequena (`request/get/post/patch`), sem estado global ou regra de domínio.

## Validação executada em 2026-08-02

- Formulários de login, cadastro, corretoras, ativos e movimentações receberam
  região `role="alert"`, alvo de erro por campo e `aria-describedby`.
- TypeScript centraliza foco no primeiro campo inválido, limpa estado de erro ao
  editar e anuncia falhas em região acessível.
- Teste de template garante presença de alvos acessíveis de erro nos fluxos
  principais.
