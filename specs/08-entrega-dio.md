# Fase 8 — Documentação e entrega DIO

- **Status:** em andamento
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-08-02

## Objetivo

Transformar o sistema validado em uma entrega reproduzível, autoral e fácil de
avaliar, demonstrando o aprendizado em vez de apenas apresentar código.

## Escopo

- **Dentro:** README, licença, diagramas mínimos, evidências de testes,
  screenshots, changelog e revisão final.
- **Fora:** material de marketing e documentação redundante.

## Tarefas

- [x] Escrever README com objetivo, arquitetura, tecnologias, pré-requisitos,
  configuração, execução, migrations e testes.
- [x] Explicar a melhoria autoral: isolamento por usuário, valores decimais,
  dashboard e segurança em camadas.
- [x] Registrar diferenças em relação ao repositório-base sem reproduzir código
  sem licença.
- [x] Incluir modelo de ameaça resumido e limitações honestas, especialmente
  preço manual e mitigação parcial de DDoS.
- [x] Adicionar screenshots sem dados/segredos reais: login, dashboard,
  corretoras, ativos e extrato.
- [x] Adicionar exemplo de API sanitizado e roteiro público de evidências.
- [x] Publicar comandos de teste, cobertura e auditoria com resultados esperados.
- [x] Escolher licença do projeto e conferir licenças das dependências.
- [ ] Fazer instalação do zero seguindo apenas o README.
- [ ] Revisar commits/PRs e garantir que nenhuma credencial ou artefato local
  entrou no histórico.

## Critérios de aceitação

- [x] O README responde exatamente aos seis itens exigidos pela DIO: o que faz,
  como executar, tecnologias, melhoria, testes e aprendizados.
- [ ] Um ambiente Linux limpo reproduz build, banco, migrations e testes.
- [ ] Pipeline final está verde e evidências não contêm dados sensíveis.
- [ ] A entrega deixa explícito o que é produção-ready e o que ainda exigiria
  infraestrutura adicional.

## Riscos e mitigação

- **Risco:** documentação divergir do sistema. → **Mitigação:** dry-run final
  baseado somente no README e revisão no mesmo PR de cada mudança.
