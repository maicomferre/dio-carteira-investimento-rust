# Auditoria de Segurança

Última execução manual registrada: 2026-07-29.

## Comandos executados

- `npm audit --audit-level=moderate`
- `cargo audit`
- `trivy --version`
- `trivy fs --scanners vuln,secret,misconfig --skip-dirs target --skip-dirs node_modules --skip-dirs .git --severity HIGH,CRITICAL --exit-code 0 .`
- `npm run audit:supply-chain`
- varredura local por nomes e padrões sensíveis em arquivos versionáveis.

## Resultado registrado

- `npm audit`: 0 vulnerabilidades reportadas.
- `cargo audit`: inicialmente encontrou `RUSTSEC-2023-0071` em `rsa 0.9.10`,
  trazido por `jsonwebtoken`. Como a aplicação usa apenas HS256, `jsonwebtoken`
  foi removido e substituído por uma implementação interna mínima de JWT HS256
  com HMAC-SHA256. Nova execução: 0 vulnerabilidades reportadas.
- `trivy`: disponível na versão 0.52.2; a base foi atualizada antes do scan.
- `trivy fs`: nenhum achado HIGH/CRITICAL foi impresso no escopo varrido.
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
```

O script roda auditoria Rust quando `cargo-audit` estiver instalado, auditoria
npm, Trivy filesystem e geração de SBOM CycloneDX em `reports/security/`. Os
artefatos gerados são ignorados pelo Git para evitar snapshots ruidosos no
repositório público.

## Limites conhecidos

- Esta auditoria não substitui revisão manual, pentest ou validação privada do
  host.
- O scan de imagem Docker deve ser executado depois do build da imagem final.
- Regras reais de Nginx, Fail2ban, firewall, domínios, usuários e deploy do VPS
  permanecem fora deste repositório público.
