# Evidências da Entrega DIO

Este documento reúne evidências públicas e sanitizadas da aplicação Carteira de
Investimentos. Ele não deve conter domínio real, IP público, paths de VPS,
usuários do servidor, regras de proxy, Fail2ban, firewall, backups, secrets ou
prints com dados pessoais.

## Screenshots esperadas

Coloque as imagens em `docs/screenshots/` usando dados fictícios. Antes de
commitar, confirme que a barra de endereço, console, cookies e qualquer segredo
não aparecem nos prints.

Sugestão de arquivos:

- `docs/screenshots/01-login.png` — tela de login.
- `docs/screenshots/02-dashboard.png` — resumo da carteira com dados fictícios.
- `docs/screenshots/03-corretoras.png` — cadastro/listagem de corretoras.
- `docs/screenshots/04-ativos.png` — cadastro/listagem de ativos.
- `docs/screenshots/05-extrato.png` — compras, vendas e fluxo diário.

Screenshots adicionadas até agora:

- [Login](screenshots/01-login.png)
- [Dashboard](screenshots/02-dashboard.png)
- [Corretoras](screenshots/03-corretoras.png)
- [Ativos](screenshots/04-ativos.png)
- [Extrato](screenshots/05-extrato.png)

## Massa de dados fictícia

Use somente exemplos demonstrativos:

- Usuário: `Aluno DIO`.
- Corretoras: `Nubank` e `XP Investimentos`.
- Ativos: `PETR4`, `CMIG4`, `ASAI3`, `KLBN4`.
- Moedas: `BRL` e, se necessário, `USD` para ativo internacional fictício.

Não use CPF, e-mail pessoal, chave real, token real ou extrato real.

## Exemplos sanitizados de API

Os exemplos abaixo usam host local e dados fictícios. Em execução real, o cookie
de sessão e o token CSRF são emitidos pela aplicação e não devem ser versionados.

```bash
curl -i http://127.0.0.1:3000/health/live
```

Resposta esperada:

```json
{"status":"ok"}
```

Cadastro de corretora autenticado, com valores demonstrativos:

```bash
curl -i -X POST http://127.0.0.1:3000/api/brokers \
  -H 'Content-Type: application/json' \
  -H 'X-CSRF-Token: <csrf-token>' \
  -b 'session=<cookie-http-only>' \
  --data '{"name":"Nubank","country":"BR","notes":"Conta de exemplo"}'
```

Consulta do resumo da carteira:

```bash
curl -i http://127.0.0.1:3000/api/portfolio/summary \
  -b 'session=<cookie-http-only>'
```

## Evidências de validação local

Registre nesta seção o resultado dos comandos antes da entrega final:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
npm run check
npm run test:ts
npm run build
npm run audit:public-boundary
```

Resultado esperado: todos os comandos devem terminar com código `0`.

## Fronteira pública/privada

Este repositório pode documentar como rodar a aplicação localmente e quais
controles públicos existem no código. A configuração real de hospedagem Linux,
gateway HTTP, bloqueios de rede, usuários de sistema, domínio, paths e runbooks
operacionais fica fora do Git público.
