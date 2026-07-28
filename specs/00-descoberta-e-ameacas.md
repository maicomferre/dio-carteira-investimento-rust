# Fase 0 — Descoberta, decisões e modelo de ameaças

- **Status:** planejada
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-28

## Contexto e objetivo

Eliminar incertezas caras antes do scaffold definitivo. Esta fase transforma o
texto do desafio e o `SPEC.md` em contratos verificáveis, sem construir produto.

## Escopo

- **Dentro:** jornadas, modelo de domínio, endpoints, threat model, matriz ASVS,
  versões/licenças e spikes descartáveis de SQLx/Askama/JWT.
- **Fora:** telas finais e funcionalidades persistentes.

## Decisões iniciais

| Data | Decisão | Justificativa | Alternativas |
|---|---|---|---|
| 2026-07-25 | Implementação própria; base DIO só como referência | Base sem licença declarada e com escolhas didáticas inseguras | Fazer fork e refatorar |
| 2026-07-25 | Monólito modular em um crate | Separação suficiente para o porte do projeto | Microserviços; workspace precoce |
| 2026-07-25 | ASVS 5.0 L2 aplicável | Segurança é requisito primário e verificável | Usar somente Top 10 |
| 2026-07-25 | Preço de compra manual; metadados/preço indicativo podem vir de provider confirmado pelo usuário | Evita dependência cega de API externa e mantém cadastro manual como fallback | API externa obrigatória no fluxo |
| 2026-07-28 | Totais, graficos e posicoes sao filtrados por moeda | Evita soma incorreta de BRL e USD sem fonte cambial | Total consolidado com cambio automatico |
| 2026-07-28 | Corretora e obrigatoria em toda movimentacao | Permite custodiar, auditar taxas e criar graficos por corretora desde o inicio | Adicionar corretora apenas no futuro |

## Tarefas

- [x] Desenhar jornadas de cadastro, login, logout, dashboard e CRUD.
- [x] Fechar estados e invariantes de `User`, `Session`, `Broker`, `Asset` e
  `Transaction`.
- [x] Especificar rotas HTML/API, schemas, status HTTP e erros públicos.
- [x] Produzir diagrama de fluxo de dados e threat model STRIDE simplificado.
- [x] Criar matriz Top 10:2025/ASVS com controle, teste e evidência esperada.
- [x] Verificar versões estáveis iniciais e registrar checagem de licenças antes
  de aceitar dependências.
- [ ] Validar em spikes descartáveis: decimal com SQLx, cookie JWT, CSRF e
  escaping Askama. Remover os spikes após registrar resultados.
- [ ] Comparar B3 Dados Públicos, brapi e OpenFIGI quanto a cobertura, licença,
  atribuição, limites, estabilidade e qualidade; registrar provider escolhido.
- [ ] Validar busca de `PETR4`, `CMIG4`, `ASAI3` e `KLBN4`, símbolos inexistentes,
  resultado ambíguo, 429, timeout e cache expirado.
- [x] Definir política de arredondamento e moedas suportadas no MVP.

## Critérios de aceitação

- [ ] Nenhum requisito crítico permanece ambíguo.
- [ ] Cada ameaça relevante tem prevenção, detecção ou risco aceito documentado.
- [ ] Contrato HTTP e schema inicial foram revisados antes das migrations.
- [ ] Nenhuma dependência sem licença compatível entra na solução.

## Riscos e mitigação

- **Risco:** arquitetura especulativa. → **Mitigação:** traits só após os spikes
  revelarem uma fronteira real.
- **Risco:** tratar “proteção contra DDoS” como promessa absoluta. →
  **Mitigação:** separar controles da aplicação, host, proxy e provedor.
