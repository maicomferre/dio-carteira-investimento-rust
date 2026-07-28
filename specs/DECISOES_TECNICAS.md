# Decisoes tecnicas da Fase 0

> Registro das decisoes que precisam estar fechadas antes da Fase 1.

## Versoes alvo

| Item | Decisao |
|---|---|
| Rust | Stable atual no momento da implementacao, fixado por `rust-toolchain.toml`. |
| PostgreSQL | 18.4 como alvo inicial de desenvolvimento e producao. |
| Bootstrap | 5.3.8 via npm e lockfile. |
| ASVS | OWASP ASVS 5.0.0, nivel 2 para controles aplicaveis. |
| Top 10 | OWASP Top 10:2025 para matriz de risco web. |
| Docker | OWASP Docker Top 10 como baseline de container. |

## Dependencias previstas

| Area | Candidatos | Observacao |
|---|---|---|
| Web | `axum`, `tower`, `tower-http` | Manter middleware explicito e testavel. |
| Templates | `askama` | Escape padrao obrigatorio. |
| Banco | `sqlx`, `postgres`, `uuid`, `chrono/time` | Queries parametrizadas e migrations SQLx. |
| Decimal | `rust_decimal` | Dinheiro e quantidade sem ponto flutuante. |
| Auth | `argon2`, `jsonwebtoken` ou alternativa revisada | Escolha final depende de manutencao/licenca. |
| Frontend | `typescript`, `bootstrap`, `bootstrap-icons`, `sweetalert2` | Servidos localmente em producao. |
| Graficos | biblioteca npm leve a escolher | Deve aceitar dados server-side e nao exigir SPA. |

Nenhuma dependencia entra sem checagem de licenca, manutencao, vulnerabilidades
conhecidas e necessidade real.

## Provider de instrumentos

Provider real ainda nao fica fechado por nome na Fase 0 sem spike de cobertura.
A arquitetura sera:

- navegador chama somente o backend;
- backend chama provider com allowlist de host;
- adapter normaliza resposta para DTO unico;
- cache evita chamada repetida e permite `stale-if-error`;
- cadastro manual permanece sempre disponivel.

Validacoes obrigatorias do spike:

- `PETR4`, `CMIG4`, `ASAI3`, `KLBN4`;
- simbolo inexistente;
- simbolo ambiguo;
- HTTP 429;
- timeout;
- resposta fora do schema;
- cache fresco e cache expirado.

## Politica de moeda e arredondamento

- Persistencia usa decimal em escala suficiente para evitar perda de precisao.
- Calculos preservam precisao interna.
- Exibicao monetaria:
  - BRL e USD com 2 casas;
  - quantidade com ate 10 casas, removendo zeros finais;
  - percentual com 2 casas.
- Arredondamento de exibicao: half-up salvo decisao posterior melhor justificada.
- Totais sao separados por moeda. Nao existe "total geral" BRL + USD no MVP.

## Decisoes de implantacao

- Repositorio publico contem Dockerfile, build/scan generico e Compose de
  desenvolvimento.
- Deploy remoto, Nginx, Fail2ban, firewall, systemd, backup, paths, dominios,
  IPs e usuarios de servidor ficam fora do Git publico.
- Mesmo VPS so sera usado apos preflight privado de capacidade e risco.
