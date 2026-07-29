# Planos da Especificação

`SPEC.md` define **o que** o produto deve respeitar. Estes arquivos registram
**por que**, **em qual ordem** e **como provar** cada etapa. Uma fase só pode ser
marcada como concluída quando seus critérios de aceitação tiverem evidência.

## Ordem de execução

1. [`00-descoberta-e-ameacas.md`](00-descoberta-e-ameacas.md) — fechar decisões,
   ameaça, contratos e spikes antes da arquitetura definitiva.
2. [`01-fundacao.md`](01-fundacao.md) — scaffold, configuração, banco,
   migrations, erros, observabilidade básica e CI.
3. [`02-identidade-e-sessao.md`](02-identidade-e-sessao.md) — cadastro,
   autenticação, JWT/cookies, CSRF e autorização básica.
4. [`03-carteira-e-calculos.md`](03-carteira-e-calculos.md) — ativos por usuário,
   CRUD, valores decimais e totais.
5. [`04-interface-web.md`](04-interface-web.md) — Askama, Bootstrap, TypeScript,
   dashboard e acessibilidade.
6. [`05-hardening-e-testes.md`](05-hardening-e-testes.md) — ASVS, Top 10,
   abuso/DoS, auditorias e testes adversariais.
7. [`06-containers-e-isolamento.md`](06-containers-e-isolamento.md) — Docker,
   OWASP Docker Top 10, redes, recursos e decisão de VPS.
8. [`07-producao-linux.md`](07-producao-linux.md) — proxy TLS, deploy, backup,
   restauração e operação.
9. [`08-entrega-dio.md`](08-entrega-dio.md) — documentação, evidências e entrega.

O mapa funcional inicial está em
[`ROTAS_E_TELAS.md`](ROTAS_E_TELAS.md).

Documentos de apoio da Fase 0:

- [`DOMINIO_E_INVARIANTES.md`](DOMINIO_E_INVARIANTES.md) — regras de negocio,
  entidades, value objects e calculos.
- [`CONTRATO_HTTP.md`](CONTRATO_HTTP.md) — convencoes HTTP, payloads, status e
  erros publicos.
- [`AMEACAS_E_CONTROLES.md`](AMEACAS_E_CONTROLES.md) — STRIDE, Top 10, Docker e
  riscos aceitos.
- [`ASVS_CHECKLIST.md`](ASVS_CHECKLIST.md) — matriz de evidências ASVS, OWASP
  Top 10 Web e OWASP Docker Top 10.
- [`DECISOES_TECNICAS.md`](DECISOES_TECNICAS.md) — versoes alvo, dependencias,
  provider, arredondamento e implantacao.

Documentos de evidência e operação pública:

- [`SECURITY_AUDIT.md`](SECURITY_AUDIT.md) — auditoria manual/repetível de
  dependências, filesystem, SBOM e segredos.
- [`SECURITY_EVENTS.md`](SECURITY_EVENTS.md) — eventos públicos de segurança e
  operação emitidos pela aplicação, sem regras privadas de alerta.

Testes acompanham cada fase. A fase 5 valida o conjunto e fecha lacunas, não
adia segurança para o fim. Decisões novas entram na tabela do plano afetado e,
se normativas, também no `SPEC.md`.
