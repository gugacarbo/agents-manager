---
status: accepted
date: 2026-11-27
builds-on: [ADR-0001, ADR-0004, ADR-0005]
superseded-by: null
deciders: []
---

# Isolar a autenticação do Claude Code por perfil

## Context and Problem

O aplicativo precisa representar múltiplas contas locais do Claude Code sem assumir controle sobre a
autenticação da CLI nem interferir no login padrão usado fora do aplicativo. O diretório de um
perfil pode conter estado privado da CLI e não deve ser tratado como uma configuração editável.

## Decision Drivers

- Separar o estado de autenticação por perfil sem alterar a configuração padrão.
- Manter autenticação e credenciais sob responsabilidade da CLI oficial.
- Limitar o que cruza o IPC e impedir que o estado privado entre nas rotinas de edição e
  persistência do aplicativo.

## Considered Options

### Option 1 — Isolar cada perfil e delegar a autenticação à CLI

**Pros:** Reusa o login e o status oficiais e separa o estado do perfil do ambiente padrão.
**Cons:** Depende de uma versão compatível da CLI e exige validar o isolamento no Linux.

### Option 2 — Usar o diretório padrão com alternância de login

**Pros:** Dispensa provisionar diretórios por perfil. **Cons:** Acopla o app ao login de uso diário
e não permite manter perfis independentes.

### Option 3 — Gerenciar autenticação e credenciais no aplicativo

**Pros:** Dá ao app controle direto do ciclo de vida. **Cons:** Exige custodiar segredos e duplicar
a integração de autenticação da CLI, ampliando a superfície de exposição.

## Decision

O backend Rust resolve um diretório local privado e distinto para cada perfil, separado do diretório
padrão. Define `CLAUDE_CONFIG_DIR` apenas no processo filho da CLI oficial, tanto para
`claude auth login` quanto para `claude auth status`. Não muda o ambiente global nem recorre ao
diretório padrão se o perfil não puder ser resolvido. Credenciais herdadas do ambiente não podem
substituir o login do perfil. A CLI é dona do fluxo interativo, do armazenamento e do ciclo de vida
das credenciais.

O aplicativo persiste somente os metadados não secretos do cadastro; o IPC recebe apenas campos
necessários e explicitamente permitidos de status e erros sanitizados, nunca a saída bruta da CLI. O
backend não lê, copia nem manipula credenciais armazenadas ou códigos de autenticação. O diretório
privado da CLI não é alvo do editor, da biblioteca compartilhada, de sincronização ou de backups do
aplicativo. Dados sensíveis também não entram em logs ou diagnósticos.

## Consequences

O mesmo binário atende vários perfis sem mutação no login padrão. A CLI continua evoluindo de forma
independente, então o backend deve tratar incompatibilidade como falha, sem substituir o fluxo de
autenticação nem adotar leitura direta de arquivos privados.

## Confirmation

Na implementação, executar o DoD de
[SPEC-0006](../specs/0006-cadastrar-contas-do-claude-code-em-perfis-isolados.md). Inspecionar a
fronteira Rust/IPC, as opções de execução dos filhos e as entradas do editor e dos backups; usar
credenciais fictícias para verificar que nenhuma chega à persistência do app ou aos diagnósticos.
Confirmar manualmente no Linux que dois perfis autenticados não alteram o usuário observado no
contexto padrão.

## Notes

Referências oficiais: [comandos de autenticação](https://code.claude.com/docs/en/cli-reference) e
[diretório de configuração da CLI](https://code.claude.com/docs/en/claude-directory).
