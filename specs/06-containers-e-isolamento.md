# Fase 6 — Containers, isolamento e decisão de VPS

- **Status:** em implementação
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-08-01

## Contexto e decisão

Docker é recomendado para reproduzir o ambiente e limitar recursos, não como
fronteira absoluta de segurança. O sistema terá containers próprios para app
Rust e PostgreSQL. Compartilhar o VPS é a opção inicial; a decisão final depende
do preflight de capacidade e criticidade. A topologia e os controles efetivos do
servidor não serão documentados neste repositório público.

## Topologia pretendida

Conceitualmente, somente o gateway HTTPS alcança a aplicação e somente a
aplicação alcança o banco. Endereços, portas, redes, volumes, nomes, regras e o
Compose de produção ficam num repositório privado de infraestrutura. Este
repositório público contém apenas Dockerfile, build/scan da imagem, Compose de
desenvolvimento e exemplos neutros.

Quando hospedado no VPS existente, o container da aplicação deve se comportar
como serviço interno atrás de Nginx. A imagem pública não conhece domínio,
subdomínio, path remoto, regra de proxy, rede Docker real ou política do host.
Esses detalhes são aplicados pela camada privada, no mesmo padrão operacional
dos demais projetos, sem copiar scripts ou comentários internos para este Git.

## Fronteira entre repositórios

| Público — aplicação | Privado — infraestrutura |
|---|---|
| Dockerfile multi-stage | Compose/override de produção |
| script de build da imagem | envio e promoção no VPS |
| script genérico de scan/SBOM | Nginx, TLS e virtual hosts |
| Compose de desenvolvimento | Fail2ban, firewall e regras anti-bot |
| `.env.example` vazio | secrets e arquivos de ambiente reais |
| contrato de health/config | systemd, cron, backup e monitoramento |
| testes de hardening da imagem | IP, domínio, SSH, users, paths e inventário |
| contrato de porta/health/env | Nginx real, Fail2ban, firewall e reload |

O repositório público não deve mencionar nomes, paths ou copiar comentários e
scripts dos projetos privados. A infraestrutura privada consome a imagem como
um artefato com interface documentada, sem exigir que o repositório da aplicação
conheça o servidor.

## Gate: mesmo VPS ou outro

Manter no VPS atual somente se uma medição de pelo menos sete dias mostrar:

- memória e swap sem pressão, incluindo margem para pico e restart;
- CPU e load sem saturação sustentada;
- disco abaixo do limite operacional definido, com crescimento e backups;
- I/O aceitável durante backup, migration e restart;
- limites do novo projeto não prejudicam os três sites existentes;
- risco compartilhado aceito: kernel, Docker, Nginx, firewall e host continuam
  sendo um único domínio de falha.

Um VPS separado é indicado quando qualquer gate falhar, quando os sites atuais
forem mais críticos que o projeto, ou quando for necessário conter um
comprometimento de host. Ser Rust em vez de Laravel não influencia a decisão.

## Controles OWASP Docker Top 10

- **D01 — Secure User Mapping:** imagem roda como usuário non-root da base
  distroless.
- **D02 — Patch Management:** base mínima suportada, rebuild periódico e SLA
  para CVEs; ferramentas de build ficam somente no estágio builder.
- **D03 — Network Segmentation:** redes por finalidade; DB interno; app não
  diretamente roteável pela Internet; sem acesso ao Docker socket. Detalhes da
  ligação com o gateway pertencem à infraestrutura privada.
- **D04 — Secure Defaults:** `cap_drop: ALL`, `no-new-privileges`, seccomp
  padrão/restrito, rootfs read-only e `tmpfs` explícito.
- **D05 — Security Contexts:** sem `privileged`, host PID/IPC/network ou mounts
  amplos; volumes com permissões mínimas.
- **D06 — Protect Secrets:** suporte a arquivos `*_FILE`; segredo nunca em
  imagem, build arg, log, Git ou configuração pública.
- **D07 — Resource Protection:** limites de CPU, memória, PIDs, file descriptors,
  conexões, pool e tamanho/tempo de requisição.
- **D08 — Image Integrity:** tags imutáveis/digests, checksum, SBOM, scan Trivy
  e assinatura/proveniência quando disponível.
- **D09 — Immutable Paradigm:** imagem sem mutação em runtime; dados somente em
  volumes definidos; deploy cria nova versão.
- **D10 — Logging:** stdout/stderr estruturado, rotação e limites; logs sem
  secrets e coletáveis pelo host.

## Tarefas

- [x] Produzir Dockerfile multi-stage; imagem final mínima, non-root e sem Cargo,
  npm, shell ou package manager.
- [x] Criar `.dockerignore` para reduzir contexto e impedir envio acidental de
  arquivos locais, relatórios e materiais privados ao build.
- [x] Criar somente Compose de desenvolvimento neutro; o Compose de produção é
  criado e mantido no repositório privado de infraestrutura.
- [ ] Definir `init`, stop grace, restart policy, CPU, memória, PIDs, rootfs,
  tmpfs, capabilities, logging e volumes no Compose/host privado.
- [x] Definir contrato público de porta interna `3000` e health endpoint
  `/health/live`; o mecanismo real de healthcheck fica na infraestrutura
  privada para não revelar topologia.
- [ ] Separar job/container de migration com credencial própria da aplicação.
  - [x] Contrato público e desenvolvimento local usam credenciais separadas:
    `DATABASE_MIGRATION_URL` para migrations e `DATABASE_URL` para runtime.
  - [ ] Job/container real de migration em produção fica no repositório privado
    de infraestrutura.
- [ ] Escanear imagem e filesystem com Trivy; gerar SBOM e revisar licenças.
  - [x] Filesystem scan e SBOM público/repetível via
    `npm run audit:supply-chain`.
  - [x] Build e scan da imagem final local via `npm run audit:container`.
- [x] Criar auditoria automática de `docker inspect` contra o baseline público.
- [ ] Testar restauração, restart, OOM controlado e indisponibilidade do DB.
- [ ] Medir o VPS real e registrar a decisão same-host/separate-host em
  documentação privada, sem inventário sensível no Git público.
- [x] Criar gate que rejeite arquivos/padrões de Nginx, Fail2ban, firewall,
  deploy remoto, chaves, `.env` e identificadores conhecidos da infraestrutura.

## Critérios de aceitação

- [ ] Teste privado confirma que somente o gateway alcança o app e que o DB não
  é acessível externamente; o resultado público não revela a topologia.
- [ ] App roda non-root, sem capabilities, sem Docker socket e com limites.
- [x] Imagem não contém segredo, source tree desnecessária ou ferramentas de build.
- [x] Scan não possui vulnerabilidade alta/crítica com correção disponível.
- [x] O checklist público D01, D02, D08 e D09 aponta para configuração e teste
  verificáveis; D03, D04, D05, D07 e D10 dependem também da configuração
  privada do runtime/host.
- [ ] A escolha de VPS é suportada por métricas, não pela linguagem utilizada.

## Riscos e mitigação

- **Risco:** container ser confundido com VM. → **Mitigação:** registrar o raio
  compartilhado e usar outro VPS quando isolamento de host for requisito.
- **Risco:** duplicar o toolkit privado e revelar a infraestrutura. →
  **Mitigação:** implementar scripts genéricos próprios; configuração efetiva
  vive no repositório privado.
