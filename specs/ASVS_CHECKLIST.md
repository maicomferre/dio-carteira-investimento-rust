# Checklist ASVS e OWASP

Última revisão: 2026-07-29.

Este checklist registra os controles aplicáveis ao MVP. Ele não substitui uma
auditoria formal; serve para manter evidência objetiva entre requisito,
implementação, teste e lacuna conhecida. Detalhes reais de VPS, Nginx, Fail2ban,
firewall, domínios e deploy permanecem fora do repositório público.

## Critério de status

- **Implementado:** existe código/configuração e teste ou comando de verificação.
- **Parcial:** controle existe, mas falta evidência, cobertura ou operação real.
- **Pendente:** requisito aceito, ainda sem implementação suficiente.
- **N/A:** fora do escopo do MVP, com justificativa explícita.

## ASVS 5.0 nível 2 aplicável

| Área | Status | Controle no projeto | Evidência | Lacuna |
|---|---|---|---|---|
| Arquitetura e threat model | Implementado | SPEC, decisões técnicas, STRIDE e riscos aceitos antes da implementação | `SPEC.md`, `specs/AMEACAS_E_CONTROLES.md`, `specs/DECISOES_TECNICAS.md` | Revisar quando provider externo real for escolhido |
| Autenticação | Implementado | Argon2id, resposta genérica para credenciais inválidas, rate limit por IP + usuário | `src/application/auth.rs`, `src/infrastructure/security.rs`, testes de auth/rate limit | Sem MFA e recuperação de senha no MVP |
| Sessão | Implementado | JWT HS256 interno com issuer/audience/exp/nbf/iat, `sid`/`jti`, sessão revogável e cookie `HttpOnly` | Testes de token adulterado, expirado e audience incorreta | Rotação operacional de chaves fica em runbook privado |
| Controle de acesso | Implementado | Toda leitura/mutação usa `user_id`; banco reforça ownership com FKs compostas | `tests/security_repository.rs`, migration `20260728000600_*` | Nenhum perfil administrativo no MVP |
| Validação de entrada | Implementado | DTOs com `deny_unknown_fields`, value objects, enums allowlist, decimais validados | Testes de domínio e mass assignment em `src/presentation/http.rs` | Mensagens por campo ainda podem evoluir |
| CSRF e navegação web | Implementado | Double-submit cookie/header, validação de Origin/Referer, mutações autenticadas protegidas | Testes unitários de CSRF e teste HTTP `403` | Validar UX sem JavaScript em fase de interface |
| Saída/HTML/XSS | Implementado | Templates Askama escapam dados do usuário; CSP restritiva sem recurso remoto obrigatório | Teste de escape do dashboard e `security_headers_include_csp_and_no_store` | Revisão visual/browser ainda pendente |
| Criptografia e segredos | Parcial | Secrets obrigatórios >=32 chars, Argon2id e HMAC-SHA256; segredos fora do Git | `src/infrastructure/config.rs`, `cargo audit`, script de scan | TLS, rotação e armazenamento real são privados de infraestrutura |
| Erros e logging | Parcial | `AppError` gera envelope público estável com `correlation_id`; logs usam tracing sem secrets intencionais | Testes HTTP `401/403/422/429/500/503` | Eventos e alertas operacionais ainda pendentes |
| Disponibilidade | Parcial | Body limit, timeout, pool limit, concorrência, rate limits global/login/registro/mutação | Testes de rate limit `429` e readiness `503`; config tipada | Teste controlado de slow request, saturação de DB e concorrência ainda pendente |
| Banco e integridade | Implementado | PostgreSQL, SQLx parametrizado, migrations versionadas, constraints de ownership | Migrations e testes SQLi/IDOR/ownership | Backup/restore real pertence à operação privada |
| API e contrato | Parcial | Status HTTP previsíveis, envelope de erro com `correlation_id`, CSRF em mutações e autenticação por cookie | `specs/CONTRATO_HTTP.md`, testes HTTP de status | Erros por campo ainda não estão completos no JSON real |
| Supply chain | Implementado | Lockfiles, `cargo audit`, `npm audit`, Trivy filesystem, SBOM CycloneDX ignorado pelo Git | `scripts/audit-supply-chain.sh`, `specs/SECURITY_AUDIT.md` | Licença formal do projeto ainda pendente para entrega DIO |
| Arquivos e upload | N/A | O MVP não aceita upload de arquivos do usuário | Rotas documentadas em `specs/CONTRATO_HTTP.md` | Reavaliar se upload for adicionado |
| Comunicação externa | Parcial | Provider de instrumentos passa pelo backend/cache; navegador não acessa provider direto | `src/infrastructure/instrument_provider.rs` | Provider externo real, retry/jitter/circuit breaker e licença ainda pendentes |

## OWASP Top 10 Web

| Risco | Status | Evidência local |
|---|---|---|
| A01 Broken Access Control | Implementado | Testes IDOR, queries por `user_id`, FKs compostas |
| A02 Security Misconfiguration | Parcial | Config fail-closed para secrets/origins e headers seguros; falta validar operação privada |
| A03 Software Supply Chain Failures | Implementado | `cargo audit`, `npm audit`, Trivy filesystem e SBOM |
| A04 Cryptographic Failures | Parcial | Argon2id, HMAC-SHA256 e secrets fortes; TLS/rotação ficam na infra privada |
| A05 Injection | Implementado | SQLx parametrizado, DTOs restritos e teste com payload SQLi-like |
| A06 Insecure Design | Implementado | Threat model, invariantes e separação de responsabilidades documentados |
| A07 Authentication Failures | Implementado | Rate limit, resposta genérica, sessão revogável e testes JWT/cookie |
| A08 Integrity Failures | Parcial | Migrations/lockfiles/auditoria; assinatura/proveniência de imagem ainda pendente |
| A09 Logging and Alerting Failures | Parcial | Tracing e request id existem; alertas operacionais ainda não definidos |
| A10 Exceptional Conditions | Implementado | Erros tipados, resposta `500` genérica, readiness `503` e rollback transacional |

## OWASP Docker Top 10

| Controle | Status | Evidência local |
|---|---|---|
| D01 User Mapping | Implementado | Dockerfile usa usuário non-root dedicado |
| D02 Patch Management | Parcial | Base fixa e auditável; falta rotina privada de rebuild/SLA |
| D03 Network Segmentation | Parcial | Contrato público exige app atrás do gateway e DB interno; topologia real é privada |
| D04 Secure Defaults | Parcial | Requisitos documentados; auditoria automática de `docker inspect` pendente |
| D05 Security Contexts | Parcial | Sem configuração de produção pública por decisão de segurança |
| D06 Protect Secrets | Implementado | `.env` real ignorado, scan de nomes sensíveis, secrets fora de imagem/Git |
| D07 Resource Protection | Parcial | Limites da aplicação existem; limites efetivos de container ficam na infra privada |
| D08 Image Integrity | Parcial | SBOM filesystem existe; scan de imagem final e digest imutável pendentes |
| D09 Immutable Paradigm | Parcial | Dockerfile público suporta artefato imutável; deploy real privado |
| D10 Logging | Parcial | Logs stdout/stderr via tracing; coleta/retenção/alerta ficam no host privado |

## Próximas lacunas verificáveis

1. Definir eventos/níveis de log para login falho repetido, `429`, `5xx` e
   exaustão de pool, mantendo regras de alerta fora do Git público.
2. Executar build e scan Trivy da imagem final.
3. Criar teste controlado de saturação de concorrência/DB e slow request.
4. Escolher licença do projeto e registrar compatibilidade das dependências.
