# Fase 7 — Implantação e operação em Linux

- **Status:** planejada
- **Criado em:** 2026-07-25
- **Última atualização:** 2026-07-25

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

## Tarefas

- [ ] Manter Compose de produção, gateway HTTPS e proteção de entrada apenas no
  repositório privado de infraestrutura.
- [ ] Criar endpoint DNS dedicado somente na configuração privada.
- [ ] Integrar os controles compartilhados do host sem trazer scripts,
  parâmetros ou inventário para o repositório público.
- [ ] Executar como usuário sem shell/root, filesystem read-only quando viável e
  diretórios de escrita explícitos.
- [ ] Configurar TLS moderno, renovação automática e HSTS somente após HTTPS
  estar validado.
- [ ] Aplicar firewall default-deny, acesso administrativo restrito e proteção
  de força bruta no perímetro quando aplicável.
- [ ] Guardar secrets fora de imagem/repositório, com permissões mínimas.
- [ ] Definir CPU/memória/PIDs/file descriptors e política de restart.
- [ ] Executar migration como job separado antes da troca de versão.
- [ ] Fazer deploy por imagem/artefato imutável identificado pelo commit,
  smoke-test e promoção; rollback troca para a imagem anterior, nunca rebuilda.
- [ ] Manter a configuração real de deploy em repositório privado de
  infraestrutura; o público contém apenas exemplos com placeholders.
- [ ] Não versionar nem mesmo templates de Nginx, Fail2ban, firewall, systemd,
  backup ou comunicação SSH no repositório da aplicação.
- [ ] Automatizar backup PostgreSQL, retenção, criptografia e teste real de
  restauração em banco isolado.
- [ ] Documentar deploy, rollback, rotação de chave, incidente e atualização.
- [ ] Validar shutdown gracioso sem perder requisições em andamento.

## Critérios de aceitação

- [ ] Scan externo privado encontra apenas serviços intencionalmente publicados.
- [ ] HTTP redireciona para HTTPS e cookies seguros funcionam.
- [ ] Processo não roda como root e não possui credencial de migration.
- [ ] Busca no histórico Git público não encontra domínio/IP/SSH/path/inventário
  da produção nem referência aos repositórios privados.
- [ ] Restore recupera dados e aplicação inicia sobre o banco restaurado.
- [ ] Falha de DB/proxy/restart produz logs e recuperação conforme o runbook.

## Riscos e mitigação

- **Risco:** botnets escanearem IP público. → **Mitigação:** superfície mínima,
  patches, firewall, proxy, rate limits, logs/alertas e WAF/CDN opcional.
- **Risco:** backup nunca restaurável. → **Mitigação:** teste periódico de
  restauração é critério, não tarefa opcional.
