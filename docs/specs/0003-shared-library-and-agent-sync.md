---
status: accepted
date: 2026-11-26
builds-on: [ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0005]
implemented-by: []
design-ref: docs/UI-PLAN.md
---

# Gerenciar biblioteca compartilhada opcional e sincronizar configurações entre agentes

## Goal

Permitir que o usuário promova ou crie itens reutilizáveis (`MCPs`, `Skills`, `Rules`, `Settings`)
em uma **biblioteca compartilhada opcional** sob o diretório de dados XDG (`library/`) e os
sincronize seletivamente para agentes e escopos (`Global` / `Project`) com **preview obrigatório de
diff**, uso de **symlinks (com fallback explícito para cópia)** em Skills e Regras, **merge
estruturado JSON ↔ TOML** em MCPs e Settings e alerta explícito contra promoção inadvertida de
segredos literais.

## Flow

1. O usuário acessa a área `Library` para visualizar itens reutilizáveis existentes ou usa
   **`Promote to library`** a partir de qualquer configuração nativa aberta (`Global` ou `Project`).
2. Se o item promovido contiver prováveis segredos literais (`SecretSpan`), a UI exibe alerta
   explícito recomendando `${VAR}` e exige confirmação antes de gravar o item na biblioteca XDG.
3. Na biblioteca, o usuário edita o item e seleciona os destinos desejados (`AgentKind` +
   `ConfigScope` Global ou `projectId`).
4. Antes de qualquer escrita em destino, o frontend chama `preview_sync(request)` e apresenta o
   **Preview sync**: para cada destino, exibe a estratégia (`symlink`, `copy` ou `merge`), o diff
   entre o conteúdo atual em disco e o conteúdo proposto, `expectedSha256` e eventuais avisos.
5. Ao confirmar `Apply sync`, o backend grava backup rotacionado em XDG dos destinos modificados,
   cria symlinks ou executa merge atômico (`toml_edit` / JSON cirúrgico), atualiza
   `library/sync-state.json` e retorna o resultado por destino.
6. A UI exibe o status de cada destino vinculado (`In sync`, `Drifted`, `Not linked` ou
   `Broken symlink`), permitindo inspecionar divergências e ressincronizar com segurança.

## Contract

### Layout em disco da biblioteca compartilhada (XDG `ProjectDirs`)

- `library/mcps/<id>.json` — definições normalizadas de servidores MCP reutilizáveis
- `library/skills/<slug>/SKILL.md` (e arquivos auxiliares do diretório da skill)
- `library/rules/<slug>.md` — regras/instruções reutilizáveis
- `library/settings/<slug>.json` — fragmentos estruturados de settings
- `library/sync-state.json` — registro de vínculos
  `{ itemId, targetPath, strategy, lastSyncedSha256, syncedAt }`

### Tipos de domínio (`crates/core`) e IPC (`#[specta::specta]` → `src/bindings/tauri.ts`)

- `SyncStrategy`: `"symlink" | "copy" | "merge"`
- `SyncTargetStatus`: `"in-sync" | "drifted" | "not-linked" | "broken-symlink"`
- `SyncTarget`:
  `{ agent: AgentKind, scope: ConfigScope, projectId?: string, path: string, status: SyncTargetStatus }`
- `LibraryItem`:
  `{ id: string, category: ConfigCategory, name: string, sourcePath: string, hasLiteralSecrets: boolean, targets: SyncTarget[] }`
- `SyncPreviewItem`:
  `{ targetPath: string, agent: AgentKind, scope: ConfigScope, strategy: SyncStrategy, currentContent: string | null, proposedContent: string, expectedSha256: string | null, changed: boolean, warnings: string[] }`
- `SyncApplyResult`:
  `{ targetPath: string, applied: boolean, strategyUsed: SyncStrategy, backupPath?: string, error?: CoreError }`

### Commands Tauri

- `list_library_items() -> Result<Vec<LibraryItem>, CoreError>`
- `promote_to_library(request: PromoteLibraryRequest) -> Result<LibraryItem, CoreError>`
- `save_library_item(request: SaveLibraryItemRequest) -> Result<LibraryItem, CoreError>`
- `delete_library_item(item_id: String) -> Result<(), CoreError>`
- `preview_sync(request: SyncPreviewRequest) -> Result<Vec<SyncPreviewItem>, CoreError>`
- `apply_sync(request: SyncApplyRequest) -> Result<Vec<SyncApplyResult>, CoreError>`

### Garantias

- **Sem migração obrigatória:** o aplicativo funciona integralmente mesmo com `library/` vazio.
- **Preview obrigatório:** `apply_sync` exige o conjunto de `expectedSha256` obtido de
  `preview_sync`, garantindo que nenhuma escrita ocorra sem diff revisado nem sobre alterações
  externas ocorridas após o preview.
- **Symlink com fallback explícito:** Skills e Regras usam `symlink` por padrão; se o symlink falhar
  ou houver destino incompatível, o backend reporta o motivo e só realiza `copy` mediante
  confirmação explícita do usuário.

## Edge cases

| #   | WHEN ⟨trigger⟩                                                                                       | THE SYSTEM SHALL ⟨response⟩                                                                                |
| --- | ---------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| 1   | The shared library directory has no items yet                                                        | Render an empty state inviting optional creation or promotion without requiring migration                  |
| 2   | The user promotes a native item containing detected literal secrets                                  | Display an explicit warning recommending `${VAR}` and require confirmation before storing in `library/`    |
| 3   | The user applies sync for a Skill or Rule to an available target                                     | Create a symlink pointing to the XDG `library/` item and record the link in `sync-state.json`              |
| 4   | Symlink creation fails (unsupported filesystem or conflicting existing target)                       | Surface the failure and require explicit user confirmation before falling back to `copy`                   |
| 5   | The user syncs an MCP item across JSON (`Claude Code` / `Antigravity`) and TOML (`Codex`)            | Translate the normalized MCP structure JSON ↔ TOML and merge surgically using `toml_edit` / JSON merge     |
| 6   | Selected targets already match the proposed content (`changed === false`)                            | Keep `Apply sync` disabled when no target has effective changes                                            |
| 7   | A target file changes externally between `preview_sync` and `apply_sync` (`expectedSha256` mismatch) | Abort writing that target with `CoreError::ExternalConflict` while reporting per-target results            |
| 8   | A synced target file is modified outside the app after sync                                          | Report target status as `Drifted` with actions to inspect diff, resync, or unlink                          |
| 9   | A target symlink points to a missing library item or path                                            | Report target status as `Broken symlink` and offer repair or unlink actions                                |
| 10  | The user deletes a library item via `delete_library_item`                                            | Remove only the item in `library/` and list affected targets without silently deleting copied native files |

## Out of scope

- **`SPEC-0004` — Histórico de Backups, Restauração e Busca Unificada:** visualização e restauração
  de snapshots rotacionados e busca global via Command Palette (`Ctrl+K`).
- **Fora da v1:** sincronização remota em nuvem ou push automático em Git, suporte/empacotamento
  para macOS e Windows, tradução/i18n da UI para pt-BR e regressão visual por screenshots.

## Open questions

Nenhum item em aberto (decisões fechadas no brainstorm).

## Definition of Done

```bash
cargo test -p agents-manager-core     # exit 0 — layout XDG library/, tradução JSON ↔ TOML, symlinks + fallback explícito, drift, broken symlink e ExternalConflict (cases 2, 3, 4, 5, 7, 8, 9, 10)
cargo test -p agents-manager-adapters # exit 0 — resolução de destinos de sync por agente e escopo (cases 3, 5)
bun run check                         # exit 0 — Biome + tsc --noEmit + cargo fmt --check + cargo clippy --workspace
bun run check:guards                  # exit 0 — boundaries, tamanho de componente, style drift, stories, TanStack e design system
bun run test:ts                       # exit 0 — empty state da library, alerta de segredos na promoção, preview diff obrigatório, fallback de cópia e estados Drifted/Broken (cases 1, 2, 4, 6, 8, 9)
bun run test:storybook                # exit 0 — stories renderizadas em Chromium
bun run test:e2e                      # exit 0 — fluxo Library → Preview sync → Apply sync e navegação por teclado
python3 scripts/docs-check            # exit 0 — integridade CASA
```

## Human review

- Promover um MCP e uma Skill reais no app desktop (`bun run dev`), revisar o diff de `Preview sync`
  entre Claude Code (JSON) e Codex (TOML), aplicar o sync e confirmar o symlink/merge criado em
  disco.
- Validar que o alerta de segredo literal aparece claramente antes de promover um arquivo com chave
  em `env`.

## Verification

```text
(fill in on close)
```

<!-- Close checklist (single atomic commit):
     [ ] DoD green, evidence recorded above
     [ ] status: implemented + implemented-by with real paths
     [ ] new non-inferable gotchas → AGENTS.md
     [ ] new current state → relevant context chapter
     [ ] scripts/docs-check --emit-index (READMEs regenerated) -->
