---
status: proposed
date: 2026-11-27
builds-on: [ADR-0001, ADR-0003, ADR-0005]
superseded-by: null
deciders: []
---

# Usar OpenSSH no backend desktop para ambientes remotos

## Context and Problem

A resolução de caminhos dos adapters parte de um diretório base injetado, mas as operações locais
não podem interpretar caminhos de outra máquina como caminhos no desktop. É preciso escolher quem
controla a conexão, a identidade do destino e a fronteira entre os dois ambientes.

## Decision Drivers

- Reaproveitar a configuração de conexão e autenticação que o usuário já mantém no OpenSSH.
- Conservar a fronteira Rust/IPC tipada e não instalar serviço do aplicativo no host.
- Testar a descoberta e o isolamento de ambientes sem usar o home nem os hosts reais do usuário.

## Considered Options

### Option 1 — Cliente OpenSSH do sistema, encapsulado em Rust

**Pros:** Reaproveita aliases, agente de autenticação e `known_hosts`; mantém o aplicativo desktop
sem daemon próprio. **Cons:** Depende do cliente instalado e de autenticação não interativa; exige
encapsular processos, limites de tempo e classificação de falhas.

### Option 2 — Biblioteca SSH embarcada

**Pros:** Controle direto da sessão e dos erros do protocolo. **Cons:** O aplicativo precisaria
integrar ou reproduzir a resolução, autenticação e verificação de identidade do OpenSSH.

### Option 3 — Serviço próprio ou montagem remota

**Pros:** Oferece uma API própria ou uma interface de arquivos familiar. **Cons:** Introduz
provisionamento ou dependência de montagem e enfraquece a distinção entre alvos locais e remotos.

## Decision

Usar o cliente OpenSSH do sistema por trás de um transporte Rust injetável. O WebView recebe apenas
IPC de operações do produto: não recebe shell, acesso genérico a arquivos ou opções SSH arbitrárias.
O backend monta argumentos estruturados sem shell local, usa verificação estrita pelo OpenSSH e não
encaminha o agente de autenticação para o servidor. Operações remotas formam um conjunto fechado;
valores externos não viram trechos de comandos.

Modelar o destino como `Local` ou um ambiente SSH registrado. O transporte resolve o usuário e o
home no contexto remoto; os adapters definem convenções dos agentes, não processos SSH. Persistir
somente metadados de cadastro sob XDG, conforme a ADR-0005, sem reescrever a configuração do
OpenSSH. A identidade do ambiente faz parte das consultas e dos resultados: não usar um caminho
sozinho como identidade e não recorrer ao ambiente local após erro remoto.

## Consequences

- O aplicativo continua local-first: usa infraestrutura SSH já mantida pelo usuário, sem backend
  próprio no servidor.
- O transporte precisa tratar timeout, cancelamento e saída de processo como dados, sem repassar
  conteúdo sensível para logs ou mensagens brutas de erro.
- A configuração OpenSSH já existente é uma fronteira de confiança do usuário, não um formato que o
  aplicativo interpreta ou altera.

## Confirmation

Revisar a fronteira IPC e executar o DoD da
[SPEC-0005](../specs/0005-cadastrar-e-inspecionar-ambientes-ssh.md), incluindo integração com
OpenSSH isolado, verificação de identidade e troca de ambiente.
