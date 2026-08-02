# Fase 7 — Implantação e operação em Linux

- **Status:** em implementação
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-08-02

## Objetivo

Provar que a aplicação pode operar com segurança e recuperação previsível em um
servidor Linux, sem transformar o projeto em uma plataforma complexa.

## Escopo

- **Dentro:** build release, usuário sem privilégio, reverse proxy TLS,
  firewall, secrets, PostgreSQL, backup/restore, health checks e runbook.
- **Fora:** Kubernetes, alta disponibilidade multi-região e CDN obrigatória.

## Confidencialidade operacional

A arquitetura deve garantir gateway HTTPS, aplicação e banco em fronteiras
mínimas, mas a topologia concreta é privada. Portas, endereços, regras, nomes,
serviços do host e procedimentos de acesso não serão descritos neste
repositório. Os limites equivalentes continuam obrigatórios na aplicação mesmo
quando também existirem na infraestrutura.

## Contrato público com Nginx

Em produção, a aplicação Rust deve ficar atrás de Nginx ou gateway equivalente.
Este repositório só documenta o contrato que a aplicação oferece para esse
gateway:

- escutar apenas em endereço privado/local definido por ambiente;
- expor `GET /health/live` para vida do processo e `GET /health/ready` para
  dependências essenciais;
- aceitar `X-Request-Id` recebido do proxy ou gerar um identificador próprio;
- exigir `APP_TRUSTED_PROXY_IPS` fora de desenvolvimento e aceitar
  `X-Real-IP` somente quando a conexão vier de um desses IPs exatos;
- exigir que o gateway sobrescreva `X-Real-IP` com o endereço do cliente e
  remova `X-Forwarded-For`; a aplicação ignora este último;
- emitir logs estruturados em stdout/stderr, sem tokens, cookies ou segredos;
- aplicar limites internos de corpo, timeout e rate limit como segunda camada;
- receber tráfego HTTPS já terminado pelo gateway e usar cookies seguros em
  produção;
- manter respostas previsíveis para `401`, `403`, `404`, `422`, `429` e `5xx`.

Essa lista é uma allowlist de pares imediatos, não de clientes nem de redes
CIDR. Ela deve conter apenas o endereço privado pelo qual o gateway realmente
alcança a aplicação. O endereço concreto permanece na configuração privada.

Arquivos reais de vhost, `limit_req`, bloqueio de probes, Fail2ban, firewall,
TLS, usuários, paths, SSH, rollback e reload do servidor pertencem ao
repositório privado de infraestrutura. O padrão operacional a seguir é o mesmo
dos projetos existentes no VPS: borda Nginx compartilhada, hardening no host,
logs por serviço, deploy atômico e secrets fora da release.

## Tarefas

- [ ] Manter Compose de produção, gateway HTTPS e proteção de entrada apenas no
  repositório privado de infraestrutura.
- [ ] Criar endpoint DNS dedicado somente na configuração privada.
- [ ] Integrar os controles compartilhados do host sem trazer scripts,
  parâmetros ou inventário para o repositório público.
  - [x] Contrato público de evidência para slow requests, probes e proteção de
    borda definido em [`EDGE_PRIVATE_VALIDATION.md`](EDGE_PRIVATE_VALIDATION.md).
- [ ] Adaptar a infraestrutura privada ao contrato acima, usando Nginx como
  reverse proxy e mantendo o app Rust inacessível diretamente pela Internet.
- [ ] Seguir deploy atômico privado com artefato imutável, diretório
  compartilhado para secrets/dados persistentes e troca controlada de versão.
- [ ] Executar como usuário sem shell/root, filesystem read-only quando viável e
  diretórios de escrita explícitos.
- [ ] Configurar TLS moderno, renovação automática e HSTS somente após HTTPS
  estar validado.
- [ ] Aplicar firewall default-deny, acesso administrativo restrito e proteção
  de força bruta no perímetro quando aplicável.
- [ ] Guardar secrets fora de imagem/repositório, com permissões mínimas.
- [ ] Definir CPU/memória/PIDs/file descriptors e política de restart.
- [ ] Executar migration como job separado antes da troca de versão.
  - O contrato público exige credencial separada da aplicação; comando, usuário,
    path e orquestração reais ficam privados.
- [ ] Fazer deploy por imagem/artefato imutável identificado pelo commit,
  smoke-test e promoção; rollback troca para a imagem anterior, nunca rebuilda.
- [ ] Manter a configuração real de deploy em repositório privado de
  infraestrutura; o público contém apenas exemplos com placeholders.
- [ ] Não versionar nem mesmo templates de Nginx, Fail2ban, firewall, systemd,
  backup ou comunicação SSH no repositório da aplicação.
  - [x] Gate público `npm run audit:public-boundary` bloqueia esses artefatos e
    padrões antes de commit/publicação. Ele é intencionalmente genérico para não
    revelar a infraestrutura real.
- [ ] Automatizar backup PostgreSQL, retenção, criptografia e teste real de
  restauração em banco isolado.
- [ ] Documentar deploy, rollback, rotação de chave, incidente e atualização.
  A documentação pública deve ficar limitada ao contrato; o runbook concreto
  fica privado.
- [ ] Validar shutdown gracioso sem perder requisições em andamento.

## Critérios de aceitação

- [ ] Scan externo privado encontra apenas serviços intencionalmente publicados.
- [ ] HTTP redireciona para HTTPS e cookies seguros funcionam.
- [ ] Processo não roda como root e não possui credencial de migration.
  - Em desenvolvimento, `cargo run` usa apenas `DATABASE_URL`; migrations usam
    `DATABASE_MIGRATION_URL` via `npm run db:migrate`.
- [ ] Busca no histórico Git público não encontra domínio/IP/SSH/path/inventário
  da produção nem referência aos repositórios privados.
- [ ] Restore recupera dados e aplicação inicia sobre o banco restaurado.
- [ ] Falha de DB/proxy/restart produz logs e recuperação conforme o runbook.
  - A evidência pública pode indicar apenas aprovado/reprovado, data e hash do
    artefato; saídas brutas e comandos operacionais ficam privados.

## Riscos e mitigação

- **Risco:** botnets escanearem IP público. → **Mitigação:** superfície mínima,
  patches, firewall, proxy, rate limits, logs/alertas e WAF/CDN opcional.
- **Risco:** backup nunca restaurável. → **Mitigação:** teste periódico de
  restauração é critério, não tarefa opcional.
