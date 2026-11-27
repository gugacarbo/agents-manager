---
status: proposed
date: 2026-11-26
builds-on: []
superseded-by: null
deciders: [gugacarbo]
---

# Adotar Tauri v2, Cargo Workspace em Rust e IPC tipado com tauri-specta

## Context and Problem

O `agents-manager` precisa de uma interface desktop rica (editores estruturados, visualização de
diff, busca rápida) combinada a um núcleo confiável para I/O em disco, parsing/escrita segura de
múltiplos formatos, file watching e backups locais sem alto consumo de memória.

## Decision Drivers

- Segurança e confiabilidade no acesso ao sistema de arquivos local sem expor permissões amplas ao
  WebView.
- Produtividade no desenvolvimento de UI aproveitando experiência prévia com TypeScript e React.
- Binário leve e baixo consumo de RAM para um utilitário desktop de uso recorrente.
- Contrato ponta-a-ponta tipado entre backend e frontend para evitar _drift_ silencioso de schemas.
- Separação entre lógica de domínio/adapters e shell desktop para possibilitar futura CLI ou
  contribuição open-source.

## Considered Options

### Option 1 — Tauri v2 (Cargo Workspace Rust + UI React/TypeScript/Bun) + tauri-specta

**Pros:** Combina núcleo em Rust (`crates/core`, `crates/adapters`) com UI web moderna
(React/Vite/Tailwind/Bun), mantém o shell desktop fino em `src-tauri/`, gera tipos TypeScript
automaticamente via `tauri-specta`, e restringe o WebView com capabilities mínimas e CSP explícita.

**Cons:** Exige manter duas toolchains no repositório (Rust/Cargo e TypeScript/Bun) e instalar
bibliotecas nativas de WebKitGTK/GTK3 no Linux.

### Option 2 — GUI 100% nativa em Rust (Iced, Slint, egui ou GPUI)

**Pros:** Linguagem única, binário sem WebView e controle direto de renderização.

**Cons:** Construir editores de texto ricos, visualizadores de diff e formulários dinâmicos exige
esforço bem maior na v1 e abre mão da produtividade em TypeScript/React.

### Option 3 — Electron (Node/TypeScript + React)

**Pros:** Linguagem única em backend e UI, com comportamento homogêneo entre sistemas operacionais.

**Cons:** Empacota Chromium e Node completos, resultando em binário pesado e consumo elevado de
memória para um utilitário de configurações.

## Decision

Adotar **Tauri v2** com o núcleo desacoplado em um **Cargo Workspace** (`crates/core` e
`crates/adapters`), shell desktop enxuto em `src-tauri/`, frontend React/Vite/Tailwind gerenciado
por Bun, e contrato IPC tipado gerado por `tauri-specta` para `src/bindings/tauri.ts`.

A separação em crates puros permite testar regras de domínio e adapters em milissegundos via
`cargo test` sem tocar no WebView ou no `$HOME` real, enquanto o WebView permanece isolado com
capabilities mínimas (`core:default`), CSP restritiva e validação de fronteira na UI com Zod.

## Consequences

- **Positivas:** Contrato tipado entre Rust e TypeScript verificado em compile time; núcleo
  reutilizável fora da UI; consumo reduzido de recursos.
- **Negativas:** Pipeline de build e CI precisa validar duas linguagens (`bun run check` cobre
  Biome, `tsc`, `cargo fmt` e `cargo clippy`).
- **Obrigatório:** Toda interface IPC deve ser exposta por commands anotados com `#[specta::specta]`
  em `src-tauri/` e exportada para `src/bindings/tauri.ts` sem edição manual do arquivo gerado.
- **Proibido:** Conceder acesso amplo de filesystem ou shell diretamente ao WebView via capabilities
  permissivas; todo acesso a arquivos reais passa pelo backend Rust.

## Confirmation

```bash
cargo clippy --workspace --all-targets -- -D warnings && bun run typecheck
```

## Notes

- Base inicial materializada em `Cargo.toml`, `crates/core/`, `crates/adapters/`, `src-tauri/` e
  `src/bindings/tauri.ts`.
