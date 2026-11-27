---
status: proposed
date: 2026-11-27
builds-on: [ADR-0001, ADR-0006, ADR-0008, ADR-0009]
superseded-by: null
deciders: [gugacarbo]
---

# Separar infraestrutura SSH e contas Claude em crates dedicados

## Context and Problem

Com a evolução do `agents-manager`, o crate `crates/core` (`agents-manager-core`) acumulou
dependências pesadas de sistema operacional e infraestrutura ao lado dos tipos puros de domínio e
dos motores locais de configuração atômica e estruturada.

Além de `domain.rs`, `error.rs` e dos módulos locais de escrita atômica, edição JSON/TOML, projetos,
biblioteca compartilhada e preferências, `crates/core` passou a abrigar o transporte OpenSSH e a
edição remota (`ssh.rs`, `remote_config.rs`, `remote_config.py`), o isolamento de perfis da CLI do
Claude Code no Linux (`claude_accounts`) e a leitura de cotas do Claude Desktop
(`claude_desktop_quotas`). Essa concentração acoplou o núcleo e seus consumidores a dependências
como `nix`, cliente HTTP bloqueante com TLS e primitivas Linux de descriptografia de cookies e
tokens (`aes`, `cbc`, `pbkdf2`, `sha1`, `base64`), além de controle de processos filhos do OpenSSH e
da CLI.

## Decision Drivers

- **Compilação incremental:** isolar dependências pesadas de rede, criptografia e sistema
  operacional para evitar recompilações desnecessárias ao alterar o domínio puro ou os adapters.
- **Domínio central agnóstico a sistema operacional:** manter `crates/core` livre de código
  condicional de Linux/Unix para controle de processos, HTTP ou descriptografia específica de
  plataforma.
- **Higiene estrita de dependências:** facilitar a auditoria por crate com `cargo machete` e
  `cargo deny`, evitando que dependências de infraestrutura vazem para crates que não as utilizam.
- **Estabilidade do contrato IPC e de erros:** preservar os DTOs compartilhados, `CoreError` e os
  bindings gerados por `tauri-specta` sem quebrar `crates/adapters`, `src-tauri` ou o frontend.

## Considered Options

### Option 1 — Manter tudo em `crates/core`

**Pros:** Dispensa novos manifestos `Cargo.toml` no workspace e mantém todos os módulos internos no
mesmo crate.

**Cons:** Mantém `nix`, cliente HTTP, criptografia Linux-only e processos externos acoplados ao
domínio puro e expostos indiretamente a `crates/adapters`, degradando a compilação incremental e a
higiene de dependências.

### Option 2 — Extrair `crates/remote` (`agents-manager-remote`) e `crates/claude-accounts` (`agents-manager-claude-accounts`) mantendo DTOs/erros compartilhados em `crates/core`

**Pros:** Separa o transporte OpenSSH e a edição remota em `crates/remote`, e isola perfis da CLI e
cotas do Claude Desktop em `crates/claude-accounts`; mantém `crates/core` enxuto e agnóstico a
sistema operacional; preserva `domain.rs`, `CoreError` e o contrato IPC `tauri-specta` sem alterar o
comportamento exposto ao WebView.

**Cons:** Amplia o Cargo Workspace para cinco membros e requer manter manifestos e fronteiras de
dependência entre `crates/core`, os dois novos crates e `src-tauri`.

### Option 3 — Criar um crate separado para cada módulo do `core`

**Pros:** Granularidade máxima por subdomínio (`config`, `structured`, `projects`, `library`,
`preferences`, `ssh`, contas e cotas).

**Cons:** Fragmenta excessivamente um núcleo pequeno, multiplica `Cargo.toml` e fronteiras de
visibilidade sem ganho proporcional de compilação ou isolamento de dependências.

## Decision

Extrair `crates/remote` (`agents-manager-remote`) para o transporte OpenSSH e edição remota de
configurações e `crates/claude-accounts` (`agents-manager-claude-accounts`) para contas isoladas do
Claude Code e monitoramento de cotas do Claude Desktop, mantendo os DTOs compartilhados de IPC
(`domain.rs`), `CoreError` (`error.rs`) e os módulos locais de configuração em `crates/core`
(`agents-manager-core`).

Com essa fronteira, `crates/remote` e `crates/claude-accounts` dependem de `agents-manager-core`
para reutilizar `domain.rs` e `CoreError`, enquanto `crates/core` e `crates/adapters` ficam livres
de `nix`, cliente HTTP, primitivas criptográficas Linux e execução de processos externos,
preservando integralmente a superfície IPC em `src-tauri/` e `src/bindings/tauri.ts`.

## Consequences

- **Positivas:** `crates/core` torna-se agnóstico a sistema operacional; `crates/adapters` deixa de
  transitar dependências de SSH, HTTP ou criptografia; builds incrementais e verificações com
  `cargo machete` e `cargo deny` tornam-se mais rápidos e granulares por crate.
- **Negativas:** O workspace passa a gerenciar cinco crates (`crates/core`, `crates/adapters`,
  `crates/remote`, `crates/claude-accounts`, `src-tauri`), exigindo sincronia de `Cargo.toml` e
  lints globais.
- **Obrigatório:** Manter DTOs compartilhados (`domain.rs`) e `CoreError` (`error.rs`) em
  `crates/core`; concentrar transporte OpenSSH/edição remota em `crates/remote` e perfis/cotas
  Claude em `crates/claude-accounts`; preservar os lints do workspace em todos os membros.
- **Proibido:** Introduzir dependências de cliente HTTP, descriptografia de cookies Linux, `nix` ou
  invocação de binários OpenSSH/CLI diretamente em `crates/core` ou `crates/adapters`.

## Confirmation

```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```
