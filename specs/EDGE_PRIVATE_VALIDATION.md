# Validação privada da borda Linux

Este documento define o que a infraestrutura privada deve provar antes de uma
publicação real. Ele não descreve a topologia do VPS, regras de Nginx, Fail2ban,
firewall, usuários, paths, domínios, IPs, comandos de SSH ou scripts de deploy.

## Objetivo

Validar que a borda absorve abuso comum antes de chegar ao Axum e que, se a
borda falhar ou for contornada, a aplicação ainda degrada de forma controlada.

## Controles obrigatórios na borda privada

- Terminar HTTPS e encaminhar somente para o serviço interno da aplicação.
- Bloquear acesso direto ao container/processo Rust pela Internet.
- Limitar tamanho de header, corpo, taxa por origem, conexões simultâneas e
  tempo de leitura para reduzir slowloris e varreduras automatizadas.
- Aplicar bloqueio temporário para padrões repetidos de força bruta, probes de
  paths inexistentes e excesso de `429`/`403`, quando compatível com o host.
- Preservar ou gerar `X-Request-Id` para correlação com logs da aplicação.
- Nunca registrar cookies, JWT, senha, CSRF token ou secrets em logs.

## Testes privados mínimos

Os testes abaixo devem ser executados no repositório/ambiente privado de
infraestrutura e registrados fora deste Git público:

- Slow request: cliente envia headers/corpo lentamente e a borda encerra a
  conexão antes de consumir worker indefinidamente.
- Body grande: payload acima do limite da borda é rejeitado antes do Axum.
- Rate burst: rajada por uma mesma origem recebe bloqueio/limitação previsível.
- Probe automatizado: paths comuns de bots não revelam backend, versão ou stack.
- Acesso direto: tentativa de alcançar app e banco sem passar pelo gateway falha.
- Correlação: requisição bloqueada ou repassada tem identificador rastreável nos
  logs privados sem expor dados sensíveis.

## Evidência pública permitida

No repositório público, registrar apenas:

- data da validação;
- hash/versão do artefato da aplicação testado;
- lista de controles verificados como aprovado/reprovado;
- nenhuma saída bruta de ferramentas, domínio, IP, path interno, regra de host,
  arquivo de configuração ou comando operacional.

## Relação com a aplicação

A aplicação já mantém a segunda camada: limite de corpo, timeout, rate limits,
limite de concorrência, pool limitado, readiness `503` e envelopes de erro com
`correlation_id`. A borda privada não substitui esses controles; ela reduz carga
maliciosa antes de atingir o processo Rust.
