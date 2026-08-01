# Auditoria de Segurança

Última execução manual registrada: 2026-08-01.

## Comandos executados

- `npm audit --audit-level=moderate`
- `cargo audit`
- `trivy --version`
- `trivy fs --scanners vuln,secret,misconfig --skip-dirs target --skip-dirs node_modules --skip-dirs .git --severity HIGH,CRITICAL --exit-code 0 .`
- `npm run audit:supply-chain`
- `docker build -f container/Dockerfile -t carteira-investimentos:local .`
- `docker save carteira-investimentos:local -o reports/security/carteira-investimentos-local.tar`
- `trivy image --input reports/security/carteira-investimentos-local.tar --scanners vuln,secret,misconfig --severity HIGH,CRITICAL --exit-code 0`
- `npm run audit:container-baseline`
- varredura local por nomes e padrões sensíveis em arquivos versionáveis.

## Resultado registrado

- `npm audit`: 0 vulnerabilidades reportadas.
- `cargo audit`: inicialmente encontrou `RUSTSEC-2023-0071` em `rsa 0.9.10`,
  trazido por `jsonwebtoken`. Como a aplicação usa apenas HS256, `jsonwebtoken`
  foi removido e substituído por uma implementação interna mínima de JWT HS256
  com HMAC-SHA256. Nova execução: 0 vulnerabilidades reportadas.
- `trivy`: disponível na versão 0.52.2; a base foi atualizada antes do scan.
- `trivy fs`: nenhum achado HIGH/CRITICAL foi impresso no escopo varrido.
- `trivy image`: a primeira versão runtime baseada em `debian:trixie-slim` foi
  reprovada com 28 achados HIGH/CRITICAL, principalmente pacotes de base como
  `perl-base`, `curl` e util-linux. A imagem foi alterada para
  `gcr.io/distroless/cc-debian13:nonroot`; novo scan da imagem final reportou
  0 HIGH/CRITICAL.
- `npm run audit:container-baseline`: validou usuário runtime `65532`,
  entrypoint `/usr/local/bin/carteira`, working directory `/app`, porta interna
  `3000/tcp`, `APP_BIND_ADDR=0.0.0.0:3000`, ausência de healthcheck embutido,
  ausência de shell configurado e ausência de `/bin/sh`, `/bin/bash`, `apt-get`,
  `npm`, `node` e `cargo` no runtime.
- `npm run audit:supply-chain`: concluiu com sucesso e gerou SBOM CycloneDX em
  `reports/security/sbom.cdx.json`, ignorado pelo Git.
- Varredura por nomes sensíveis: não encontrou `.env`, `.pem`, `.key` ou
  arquivos de credencial versionáveis fora dos diretórios ignorados.
- Varredura por padrões sensíveis: apontou falsos positivos esperados em código
  e `.env.example`; nenhum segredo real foi identificado nesta revisão.

## Execução repetível

Use:

```bash
./scripts/audit-supply-chain.sh
npm run audit:container
npm run audit:container-baseline
```

O primeiro script roda auditoria Rust quando `cargo-audit` estiver instalado,
auditoria npm, Trivy filesystem e geração de SBOM CycloneDX. O segundo script
faz build da imagem, exporta o tar local, executa Trivy na imagem final e roda
o baseline público. O terceiro script roda apenas o baseline de metadados da
imagem já construída. Os artefatos gerados em `reports/security/` são ignorados
pelo Git para evitar snapshots ruidosos no repositório público.

## Limites conhecidos

- Esta auditoria não substitui revisão manual, pentest ou validação privada do
  host.
- O scan de imagem Docker local não substitui assinatura, publicação por digest
  ou política privada de promoção no VPS.
- Regras reais de Nginx, Fail2ban, firewall, domínios, usuários e deploy do VPS
  permanecem fora deste repositório público.
