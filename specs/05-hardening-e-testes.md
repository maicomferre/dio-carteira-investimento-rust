# Fase 5 — Hardening, abuso e testes adversariais

- **Status:** em andamento
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-29

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

- [x] Limitar body, duração, concorrência e pool dentro da aplicação.
  - `APP_MAX_BODY_BYTES` limita o corpo aceito pelo Axum.
  - `APP_REQUEST_TIMEOUT_SECONDS` limita duração de request.
  - `APP_MAX_CONCURRENT_REQUESTS` rejeita saturação interna com `503`.
  - `DATABASE_MAX_CONNECTIONS` e `DATABASE_ACQUIRE_TIMEOUT_SECONDS` limitam pool.
  - Tamanho de headers e conexões de borda ficam na camada Nginx/gateway privada;
    a configuração concreta do VPS não é versionada neste repositório público.
- [x] Configurar rate limits separados para global, login, registro e mutações.
  - [x] Login: limite por IP + usuário normalizado, com atraso progressivo em
    falhas e bloqueio temporário.
  - [x] Registro: limite por IP antes de acessar o banco.
  - [x] Mutações de carteira: limite por IP antes de CSRF/autorização nas rotas
    `POST`/`PATCH` de corretoras, ativos e movimentações.
  - [x] Global: usar duas camadas — proxy reverso privado para absorver abuso de
    borda e middleware Axum para falha segura caso o proxy seja contornado ou
    mal configurado. O middleware interno limita por IP e isenta somente
    `/health/live` e `/health/ready`.
- [ ] Testar slow requests no proxy e saturação controlada da aplicação/DB.
  - [x] Concorrência interna: teste HTTP mantém uma rota lenta sob `#[cfg(test)]`
    ocupando o único slot e valida que a segunda requisição recebe `503`
    controlado com `correlation_id`.
  - [x] Saturação controlada de pool PostgreSQL: teste mantém a única conexão
    ocupada e valida `/health/ready` retornando `503` com envelope público.
  - [x] Contrato público de validação privada para slow requests, probes e borda
    Linux definido em [`EDGE_PRIVATE_VALIDATION.md`](EDGE_PRIVATE_VALIDATION.md).
  - [ ] Executar os testes reais de slow request no proxy privado antes de
    produção; não versionar configuração real de Nginx/Fail2ban/firewall neste
    repositório público.
- [x] Aplicar CSP, HSTS no perfil HTTPS, nosniff, referrer/frame policy e
  `Cache-Control: no-store` em conteúdo autenticado.
- [ ] Criar testes para CSRF, XSS armazenado/refletido, SQLi, IDOR, mass
  assignment, cookie/JWT e erro interno.
  - [x] CSRF: cookie/header/origin cobertos por testes unitários.
  - [x] Cookie/JWT: token adulterado, expirado e audience incorreta cobertos.
  - [x] Mass assignment: payloads JSON com campos inesperados são rejeitados por
    `deny_unknown_fields`.
  - [x] SQLi/IDOR/ownership: testes de integração validam queries parametrizadas,
    escopo por `user_id`, tentativa de update de ativo de outro usuário e
    movimentação com corretora de outro usuário.
  - [x] XSS em template Askama: teste garante escape de valores controlados pelo
    usuário no dashboard.
  - [x] Erro interno: teste garante envelope público genérico e estável.
  - [x] Cenários HTTP completos cobrem respostas públicas de `401`, `403`,
    `422`, `429`, `500` e indisponibilidade controlada `503` pelo router Axum.
- [x] Executar auditoria Rust/npm, licença, segredo e gerar SBOM.
  - `npm run audit:public-boundary`: bloqueia artefatos privados de VPS, Nginx,
    Fail2ban, firewall, deploy remoto, `.env`, chaves, tokens e scripts de host
    antes de publicar o repositório.
  - `npm audit --audit-level=moderate`: 0 vulnerabilidades.
  - `cargo audit`: removeu dependência vulnerável `rsa` via troca de JWT para
    HS256 interno; nova execução sem vulnerabilidades.
  - `trivy fs`: nenhum HIGH/CRITICAL no filesystem com secrets/misconfig/vuln.
  - SBOM CycloneDX gerado por `npm run audit:supply-chain` em diretório ignorado.
  - Evidência detalhada: [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md).
- [x] Revisar cada item ASVS aplicável com evidência; justificar `N/A`.
  - Matriz criada em [`ASVS_CHECKLIST.md`](ASVS_CHECKLIST.md), cobrindo ASVS,
    OWASP Top 10 Web e OWASP Docker Top 10 com status, evidência e lacunas.
- [x] Definir eventos para falhas repetidas de login, 429, 5xx e exaustão de pool.
  - A aplicação emite eventos estruturados para `auth.login_failed`,
    `auth.login_rate_limited`, `http.rate_limited`,
    `http.concurrency_saturated`, `http.server_error` e
    `db.readiness_failed`.
  - Métricas, thresholds, dashboards e regras concretas do host ficam privados;
    o contrato público está em [`SECURITY_EVENTS.md`](SECURITY_EVENTS.md).
- [x] Criar gate público contra vazamento operacional.
  - O gate consulta arquivos versionados e pendentes, falhando se encontrar
    material de host/deploy, scripts de Nginx/Fail2ban/firewall, comunicação
    remota, secrets ou chaves. Detalhes efetivos do VPS continuam privados.

## Critérios de aceitação

- [x] Nenhuma pendência crítica/alta conhecida.
- [x] Todos os limites implementados na aplicação retornam erro controlado, sem
  crash ou vazamento.
- [x] Matriz ASVS/Top 10 aponta para testes ou configuração verificável.
- [x] Excesso acima do rate limit da aplicação retorna `429` previsível.
- [ ] Saturação de concorrência/DB e slow request degradam de forma previsível e
  recuperam depois em teste controlado.
  - Concorrência e DB têm testes automatizados no repositório público; slow
    request depende de execução privada na borda Linux.

## Riscos e mitigação

- **Risco:** rate limit distribuído ficar inconsistente em múltiplas instâncias. →
  **Mitigação:** V1 é uma instância; documentar Redis/proxy compartilhado antes
  de escalar horizontalmente.
- **Risco:** scanner gerar falsa confiança. → **Mitigação:** combinar automação,
  revisão manual, threat model e testes de autorização.
