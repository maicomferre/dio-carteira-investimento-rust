# Fase 5 — Hardening, abuso e testes adversariais

- **Status:** em andamento
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-28

## Objetivo

Verificar de forma sistemática os controles aplicáveis do ASVS 5.0 nível 2 e do
Top 10:2025, além de resiliência a abuso e falhas externas.

## Escopo

- **Dentro:** middleware defensivo, limites, headers, testes adversariais,
  auditoria de dependências, logs de segurança e revisão da matriz.
- **Fora:** garantia contra DDoS volumétrico; pentest certificado.

## Checklist por risco OWASP Top 10:2025

- **A01 Acesso:** deny-by-default, ownership no SQL, testes IDOR e método/rota.
- **A02 Configuração:** produção fail-closed, debug desligado, headers, portas
  mínimas, sem listagem ou mensagens detalhadas.
- **A03 Supply chain:** lockfiles, licenças, advisories, SBOM e atualizações
  controladas.
- **A04 Criptografia:** TLS, Argon2id, chaves fortes/rotacionáveis, nenhum
  segredo no repositório.
- **A05 Injeção:** SQL parametrizado, escaping Askama, validação allowlist e
  nenhuma shell construída com entrada.
- **A06 Design:** threat model, abuso por fluxo e limites de negócio.
- **A07 Autenticação:** respostas genéricas, throttling, sessão revogável.
- **A08 Integridade:** migrations e artefatos controlados; CI protegida.
- **A09 Logging:** eventos de auth/acesso negado/rate limit sem dados sensíveis,
  correlação e alertas.
- **A10 Exceções:** timeouts, falha segura, rollback e ausência de panic.

## Tarefas

- [ ] Limitar tamanho de headers/body, duração, conexões, concorrência e pool.
- [ ] Configurar rate limits separados para global, login, registro e mutações.
  - [x] Login: limite por IP + usuário normalizado, com atraso progressivo em
    falhas e bloqueio temporário.
  - [x] Registro: limite por IP antes de acessar o banco.
  - [x] Mutações de carteira: limite por IP antes de CSRF/autorização nas rotas
    `POST`/`PATCH` de corretoras, ativos e movimentações.
  - [ ] Global: usar duas camadas — proxy reverso privado para absorver abuso de
    borda e middleware Axum para falha segura caso o proxy seja contornado ou
    mal configurado.
- [ ] Testar slow requests no proxy e saturação controlada da aplicação/DB.
- [x] Aplicar CSP, HSTS no perfil HTTPS, nosniff, referrer/frame policy e
  `Cache-Control: no-store` em conteúdo autenticado.
- [ ] Criar testes para CSRF, XSS armazenado/refletido, SQLi, IDOR, mass
  assignment, cookie/JWT e erro interno.
- [ ] Executar auditoria Rust/npm, licença, segredo e gerar SBOM.
- [ ] Revisar cada item ASVS aplicável com evidência; justificar `N/A`.
- [ ] Definir alertas para falhas repetidas de login, 429, 5xx e exaustão de pool.
  Métricas e regras concretas do host ficam privadas; a aplicação pública deve
  apenas emitir eventos suficientes para a camada operacional correlacionar.

## Critérios de aceitação

- [ ] Nenhuma pendência crítica/alta conhecida.
- [ ] Todos os limites retornam erro controlado, sem crash ou vazamento.
- [ ] Matriz ASVS/Top 10 aponta para testes ou configuração verificável.
- [ ] Carga acima do limite degrada de forma previsível e recupera depois.

## Riscos e mitigação

- **Risco:** rate limit distribuído ficar inconsistente em múltiplas instâncias. →
  **Mitigação:** V1 é uma instância; documentar Redis/proxy compartilhado antes
  de escalar horizontalmente.
- **Risco:** scanner gerar falsa confiança. → **Mitigação:** combinar automação,
  revisão manual, threat model e testes de autorização.
