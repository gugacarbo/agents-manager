---
status: proposed
date: 2026-11-26
builds-on: [ADR-0002, ADR-0004]
superseded-by: null
deciders: [gugacarbo]
---

# Persistir estado, biblioteca compartilhada e backups rotacionados em arquivos XDG

## Context and Problem

Além de ler e editar arquivos nativos de cada agente, o `agents-manager` precisa armazenar a lista
de projetos registrados, a biblioteca compartilhada opcional (MCPs, Skills e Regras reutilizáveis) e
os backups gerados antes de cada escrita no disco. É preciso definir um modelo de persistência
simples, portável, sem serviços externos e que não polua repositórios Git nem diretórios nativos das
ferramentas com arquivos auxiliares.

## Decision Drivers

- Execução 100% local sem servidor de banco de dados nem daemons externos.
- Biblioteca compartilhada naturalmente compatível com symlinks de Skills/Regras e versionamento
  pessoal em Git/dotfiles.
- Backups isolados que nunca sujem `~/.claude`, `~/.codex`, `~/.gemini` nem repositórios de projeto
  com arquivos `.bak`.
- Retenção limitada para evitar crescimento indefinido do disco.

## Considered Options

### Option 1 — 100% arquivos locais em diretórios XDG + backups rotacionados em pasta própria do app

**Pros:** Armazena configurações do app, registro de projetos e biblioteca compartilhada sob os
diretórios padrão do sistema via crate `directories` (XDG no Linux), mantém Skills/Regras como
arquivos e diretórios reais prontos para symlink e versionamento em Git, e grava backups em
diretório próprio do app (`.../backups/`) com rotação por arquivo alvo (mantendo os últimos 20
snapshots).

**Cons:** Buscas complexas sobre histórico de edições futuras exigiriam leitura de índice/arquivos
ou introdução posterior de um índice embarcado.

### Option 2 — SQLite embarcado desde a v1 para registro de projetos e histórico de backups

**Pros:** Consultas relacionais rápidas e histórico estruturado em banco único.

**Cons:** Adiciona migrações, schema SQLite e camada extra de persistência antes que haja volume de
dados ou consultas que justifiquem banco relacional na v1.

### Option 3 — Arquivos `.bak` gravados ao lado de cada arquivo original

**Pros:** Implementação trivial sem mapeamento de diretórios de backup.

**Cons:** Polui diretórios nativos dos agentes e pode criar arquivos não rastreados ou
acidentalmente incluídos em commits dentro de repositórios de projeto.

## Decision

Persistir estado do aplicativo, registro de projetos, biblioteca compartilhada e snapshots de backup
**100% em arquivos locais sob diretórios XDG** (`directories`), mantendo Skills e Regras como
arquivos reais prontos para symlink e armazenando backups rotacionados (**últimos 20 snapshots por
arquivo alvo**) em diretório próprio de dados do aplicativo.

## Consequences

- **Positivas:** Zero dependência de banco de dados na v1; biblioteca compartilhada legível e pronta
  para versionamento em Git; repositórios de projeto e diretórios nativos permanecem limpos de
  arquivos `.bak`.
- **Negativas:** Um eventual histórico avançado ou busca full-text em grande escala em versões
  futuras poderá exigir a introdução de índice dedicado (ex.: SQLite) em nova ADR.
- **Obrigatório:** Todo backup gerado antes de escrita in-place ou sync deve ir para o diretório de
  backups gerenciado pelo app com política de rotação (máximo de 20 snapshots por arquivo alvo).
- **Proibido:** Gravar arquivos `.bak` ou metadados internos do `agents-manager` dentro de
  repositórios de projeto do usuário ou nas pastas nativas dos agentes.

## Confirmation

```bash
rg -n "directories" src-tauri/Cargo.toml && cargo test --workspace
```

## Notes

- A inclusão da crate `directories` em `crates/core` ou `crates/adapters` para resolver os
  diretórios de dados/config da biblioteca e backups deve ocorrer na Spec correspondente.
