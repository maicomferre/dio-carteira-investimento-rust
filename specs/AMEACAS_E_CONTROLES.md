# Modelo de ameacas e controles

> Escopo: aplicacao web Rust, PostgreSQL, templates Askama, assets locais,
> container de app e banco em Linux atras de proxy TLS privado de infraestrutura.

## Ativos protegidos

- Credenciais, hashes de senha, tokens, cookies e sessoes.
- Carteiras, corretoras, ativos, movimentacoes e calculos financeiros.
- Segredos de runtime e providers externos.
- Integridade de migrations, imagem de container e dependencias.
- Disponibilidade razoavel do app, banco e endpoints de autenticacao.

## Fronteiras de confianca

- Navegador do usuario nunca e confiavel.
- Proxy TLS/infrastrutura e confiavel apenas apos configuracao explicita de
  trusted proxies.
- Provider externo de instrumentos e nao confiavel: resposta e validada,
  limitada, cacheada e confirmada pelo usuario.
- Banco protege integridade com constraints, mas regra de negocio continua no
  dominio.

## Fluxo de dados

```text
Browser
  -> reverse proxy TLS privado
  -> Axum middleware: request id, limites, sessao, CSRF, headers
  -> presentation: rotas HTML/API, DTOs e validacao de borda
  -> application: casos de uso, autorizacao e transacoes logicas
  -> domain: invariantes, decimal e calculos puros
  -> infrastructure: SQLx/PostgreSQL, JWT, provider/cache
```

O provider externo de instrumentos so e acessado pela infraestrutura do backend.
O navegador nunca recebe token de provider, host interno, string de conexao ou
detalhe operacional.

## STRIDE simplificado

| Ameaca | Exemplo | Controle minimo | Evidencia |
|---|---|---|---|
| Spoofing | Roubo/reuso de sessao | Cookie `HttpOnly`, `Secure`, `SameSite`, JWT curto, revogacao por `sid` | Teste de expiracao/logout |
| Tampering | Alterar ativo de outro usuario | Ownership em caso de uso e query, `user_id` obrigatorio | Teste IDOR |
| Repudiation | Usuario nega movimentacao | Log estruturado com correlation ID e timestamps | Teste/log de acao critica |
| Information disclosure | Erro revela SQL/segredo | Envelope publico estavel e logs internos redigidos | Teste 500/validacao |
| Denial of service | Login brute force, payload grande | Rate limit, body limit, timeout, pool limitado | Teste 429/body |
| Elevation of privilege | Forjar role/ID no payload | Claims validadas e autorizacao no servidor | Teste payload com `user_id` falso |

## OWASP Top 10:2025

| Risco | Controle no MVP | Teste esperado |
|---|---|---|
| A01 Broken Access Control | Ownership em todas as consultas e casos de uso | Usuario B nao le/altera dados de A |
| A02 Security Misconfiguration | Config tipada, defaults fechados, headers seguros | Startup falha com config invalida |
| A03 Software Supply Chain Failures | Lockfiles, audit, deny, scan de imagem | CI bloqueia vulnerabilidade relevante |
| A04 Cryptographic Failures | Argon2id, cookies seguros, secrets externos | Cookie sem flags falha teste |
| A05 Injection | SQLx parametrizado, sem concatenacao SQL | Payload malicioso nao altera query |
| A06 Insecure Design | Threat model e invariantes antes do codigo | Checklist revisado na Fase 0 |
| A07 Authentication Failures | Cadastro separado, rate limit, respostas genericas | Brute force recebe 429 |
| A08 Integrity Failures | Migrations revisadas, imagem assinavel/digest | Build reproduzivel e scanner |
| A09 Logging/Alerting Failures | Correlation ID, logs sem PII sensivel | Falha critica gera log rastreavel |
| A10 Exceptional Conditions | Erros tipados e transacoes com rollback | Falha parcial nao grava estado invalido |

## ASVS 5.0 nivel 2 resumido

| Area ASVS | Aplicacao no projeto | Evidencia |
|---|---|---|
| Arquitetura | SPEC, invariantes e threat model antes do scaffold | Revisao da Fase 0 |
| Autenticacao | Argon2id, resposta generica, rate limit e sessao revogavel | Testes da Fase 2 |
| Sessao | Cookie seguro, JWT curto, `sid`/`jti`, logout server-side | Testes de expiracao e revogacao |
| Controle de acesso | `user_id` em todo caso de uso e query | Testes IDOR e ownership |
| Validacao | DTOs restritos, limite de corpo, decimal e enums | Testes 400/422 |
| Criptografia | Secrets externos e algoritmos configurados por allowlist | Teste de startup/config |
| Erros e logs | Erro publico estavel e log estruturado redigido | Testes 500 e auditoria de log |
| Dados | PostgreSQL com constraints, migrations e backup testado | Testes de migration/restauracao |
| Comunicacao | HTTPS no proxy, CORS fechado e CSRF em mutacoes | Testes de headers/CSRF |
| Configuracao | Defaults seguros e separacao publico/privado | CI e revisao de arquivos |
| Codigo malicioso | Lockfiles, auditoria, SBOM e scan de imagem | Pipeline da Fase 5/6 |
| API | Contrato `/api/v1`, status previsiveis e rate limits | Testes de contrato |

## OWASP Docker Top 10

- Imagem minima, atualizada e escaneada.
- Usuario nao-root e filesystem preferencialmente read-only em producao.
- Sem secrets na imagem, no Dockerfile ou no Compose publico.
- Rede separada; banco sem exposicao publica.
- Limites de CPU/memoria/processos definidos no ambiente privado.
- Logs em stdout/stderr sem dados sensiveis.
- Contexto de build minimo com `.dockerignore`.

## Riscos aceitos no MVP

- Sem MFA e recuperacao de senha por e-mail.
- Sem protecao completa contra DDoS volumetrico; isso depende de provedor,
  proxy, firewall e possivel CDN/WAF fora do repositorio publico.
- Sem conversao cambial consolidada.
