---
status: accepted
date: 2026-11-26
builds-on: [ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0005]
implemented-by: []
design-ref: docs/UI-PLAN.md
---

# Inspecionar e editar configurações globais dos agentes com escrita segura

## Goal

Substituir a rota `/editor` de demonstração por inspeção e edição _in-place_ segura de configurações
globais reais (**MCPs**, **Skills**, **Regras/Instruções** e **Settings**) para **Claude Code**,
**Codex** e **Antigravity**, em interface **100% em inglês** e sem migração obrigatória.

Ao concluir esta Spec, o usuário no Linux abre o aplicativo, visualiza os arquivos globais nativos
de cada agente detectado, edita qualquer configuração suportada no CodeMirror com **mascaramento
visual não-destrutivo de segredos**, revisa o diff em relação ao disco e grava com segurança
(**escrita atômica**, **backup rotacionado em XDG** e **detecção de conflitos externos via SHA-256 +
watcher `notify`**).

## Flow

1. O inventário (`/`) lista os três agentes suportados (`Claude Code`, `Codex`, `Antigravity`).
   Selecionar um agente detectado abre a visão global desse agente (`/agents/$agentKind`).
2. A visão global exibe os arquivos descobertos nos quatro domínios (`MCPs`, `Skills`, `Rules`,
   `Settings`), informando o caminho nativo, o formato (`json`, `toml`, `markdown`) e o estado de
   presença (`Detected` / `Not found`).
3. Selecionar um arquivo solicita `read_config_file(path)` ao backend Rust, recebendo o conteúdo
   atual em texto, o hash `sha256` e os `secretSpans` detectados, e inicia o monitoramento passivo
   via `watch_config_file(path)` (`notify`).
4. O editor CodeMirror renderiza o arquivo aplicando **decorações visuais de mascaramento** sobre os
   `secretSpans` sem alterar o buffer real em memória, oferecendo toggle explícito `Reveal secrets`
   / `Hide secrets`.
5. Alterações no rascunho executam validação sintática conforme o formato (`json` / `toml`) e
   atualizam o painel de Diff comparando o rascunho contra o texto original carregado do disco; o
   botão `Save` só habilita quando há alterações válidas.
6. Ao confirmar `Save`, o frontend envia `save_config_file(request)` com `expected_sha256`. O
   backend verifica o hash atual em disco, gera um backup rotacionado no diretório XDG de dados
   (`backups/`, mantendo até 20 snapshots por arquivo alvo), grava via arquivo temporário no mesmo
   diretório + rename atômico e retorna o novo `sha256` e o caminho do backup.
7. Se o arquivo for alterado externamente (notificado pelo evento `config-file-changed` do watcher
   `notify` ou detectado como `ExternalConflict` no salvamento), a UI exibe um aviso explícito
   oferecendo **`Reload from disk`** ou **`Overwrite (review diff)`**, sem nunca sobrescrever
   alterações externas silenciosamente.

## Contract

### Paths globais descobertos pelos adapters (`home: &Path` injetável)

| Agente          | Categoria       | Caminho(s) nativo(s)                                                | Formato    |
| --------------- | --------------- | ------------------------------------------------------------------- | ---------- |
| **Claude Code** | Settings        | `~/.claude/settings.json`                                           | `json`     |
| **Claude Code** | MCPs            | `~/.claude.json`, `~/.claude/.claude.json`                          | `json`     |
| **Claude Code** | Rules           | `~/.claude/CLAUDE.md`                                               | `markdown` |
| **Claude Code** | Skills          | `~/.claude/skills/*/SKILL.md`                                       | `markdown` |
| **Codex**       | Settings & MCPs | `~/.codex/config.toml`                                              | `toml`     |
| **Codex**       | Rules           | `~/.codex/AGENTS.md`                                                | `markdown` |
| **Codex**       | Skills          | `~/.codex/skills/*/SKILL.md`                                        | `markdown` |
| **Antigravity** | Settings        | `~/.gemini/config/config.json` (fallback `~/.gemini/settings.json`) | `json`     |
| **Antigravity** | MCPs            | `~/.gemini/config/mcp_config.json`                                  | `json`     |
| **Antigravity** | Rules           | `~/.gemini/GEMINI.md`, `~/.gemini/config/rules/*.md`                | `markdown` |
| **Antigravity** | Skills          | `~/.gemini/config/skills/*/SKILL.md`, `~/.gemini/skills/*/SKILL.md` | `markdown` |

### Tipos de domínio (`crates/core`) e IPC (`#[specta::specta]` → `src/bindings/tauri.ts`)

- `ConfigCategory`: `"mcps" | "skills" | "rules" | "settings"`
- `ConfigFormat`: `"json" | "toml" | "markdown"`
- `ConfigFileEntry`:
  `{ agent: AgentKind, category: ConfigCategory, label: string, path: string, format: ConfigFormat, exists: boolean }`
- `SecretSpan`: `{ startByte: number, endByte: number, kind: string }` (intervalos de valores
  sensíveis detectados para mascaramento não-destrutivo na UI)
- `ConfigDocument`:
  `{ entry: ConfigFileEntry, content: string, sha256: string, secretSpans: SecretSpan[] }`
- `SaveConfigRequest`:
  `{ path: string, content: string, expectedSha256: string, forceOverwrite?: boolean }`
- `SaveConfigResult`: `{ sha256: string, backupPath: string }`
- `CoreError`:
  `NotFound | PermissionDenied | PathNotAllowed | SyntaxError | ExternalConflict | IoError`

### Commands e eventos Tauri

- `list_agent_configs(kind: AgentKind) -> Result<Vec<ConfigFileEntry>, CoreError>`
- `read_config_file(path: String) -> Result<ConfigDocument, CoreError>`
- `save_config_file(request: SaveConfigRequest) -> Result<SaveConfigResult, CoreError>`
- `watch_config_file(path: String) -> Result<(), CoreError>`
- Evento `config-file-changed`: `{ path: string, diskSha256: string | null }`

### Garantias de segurança e persistência

- **Allowlist estrita:** `read_config_file`, `save_config_file` e `watch_config_file` rejeitam com
  `CoreError::PathNotAllowed` qualquer caminho que não corresponda a um arquivo global descoberto
  pelos adapters.
- **Preservação estrutural e segredos:** O buffer editado e salvo é sempre o texto original/editado
  real; o mascaramento visual nunca altera os bytes gravados em disco.
- **Backups XDG:** Backups são gravados exclusivamente sob o diretório de dados XDG do aplicativo
  (`directories`, subpasta `backups/`), mantendo no máximo **20 snapshots por arquivo alvo**. É
  proibido criar arquivos `.bak` dentro de `~/.claude`, `~/.codex` ou `~/.gemini`.

## Edge cases

| #   | WHEN ⟨trigger⟩                                                                                                | THE SYSTEM SHALL ⟨response⟩                                                                                                                            |
| --- | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | A supported agent directory is missing during detection                                                       | Display the agent as `Not detected` in neutral state without surfacing an error                                                                        |
| 2   | An optional native config file (such as `CLAUDE.md`) does not exist yet                                       | List the entry with status `Not found` without blocking inspection or editing of the remaining files                                                   |
| 3   | The draft in CodeMirror has invalid JSON or TOML syntax                                                       | Show an inline syntax error, keep `Save` disabled, and perform no disk write                                                                           |
| 4   | Literal secrets (tokens, API keys, passwords in `env`/headers) are detected in a loaded file                  | Mask those spans visually in CodeMirror by default, toggle visibility via `Reveal secrets` / `Hide secrets`, and write the real unmasked bytes on save |
| 5   | The watched file changes on disk externally (`notify` emits `config-file-changed`)                            | Display an explicit external conflict banner offering `Reload from disk` and `Overwrite (review diff)`                                                 |
| 6   | Disk content hash diverges from `expectedSha256` at `save_config_file` without `forceOverwrite`               | Reject the write with `CoreError::ExternalConflict`, skip backup creation, and prompt conflict resolution in the UI                                    |
| 7   | An I/O error occurs while writing the temporary file or renaming atomically                                   | Abort the save with `CoreError::IoError` leaving the original target file intact without partial corruption                                            |
| 8   | More than 20 saves are performed on the same target file                                                      | Rotate backups under XDG `backups/` keeping only the 20 most recent snapshots and creating zero `.bak` files in native agent directories               |
| 9   | A frontend or IPC caller requests reading, writing, or watching a path outside the adapters' global allowlist | Reject the operation in Rust with `CoreError::PathNotAllowed`                                                                                          |
| 10  | The desktop window runs at compact viewport (`900 × 600`)                                                     | Switch the side-by-side Editor/Diff layout into keyboard-accessible tabs (`Editor` / `Diff`) while preserving the active draft                         |

## Out of scope

Mapa sequencial planejado para as próximas Specs da v1 (explicitamente fora desta Spec):

- **`SPEC-0002` — Escopo de Projetos e Editores Estruturados:** registro de repositórios de projeto,
  descoberta de configurações por projeto e formulários estruturados por domínio (MCPs, Settings).
- **`SPEC-0003` — Biblioteca Compartilhada Opcional e Sincronização:** promoção de itens para
  biblioteca XDG, symlinks com fallback de cópia para Skills/Regras, merge estruturado JSON ↔ TOML
  para MCPs/Settings e preview de diff de sync.
- **Fora da v1:** suporte/empacotamento para macOS e Windows, tradução/i18n da UI para pt-BR,
  TanStack MCP e testes de regressão visual por screenshots.

## Open questions

Nenhum item em aberto (decisões fechadas no brainstorm).

## Definition of Done

```bash
cargo test -p agents-manager-core     # exit 0 — SHA-256, escrita atômica, rotação de 20 backups XDG, CoreError e detecção de SecretSpan (cases 3, 4, 6, 7, 8)
cargo test -p agents-manager-adapters # exit 0 — descoberta global injetável de Claude Code, Codex e Antigravity + allowlist de paths (cases 1, 2, 9)
bun run check                         # exit 0 — Biome + tsc --noEmit + cargo fmt --check + cargo clippy --workspace
bun run check:guards                  # exit 0 — boundaries, tamanho de componente, style drift, stories, TanStack e design system
bun run test:ts                       # exit 0 — mascaramento visual não-destrutivo no CodeMirror, validação sintática, resolução de conflito e estados de UI (cases 1, 2, 3, 4, 5, 6, 10)
bun run test:storybook                # exit 0 — stories renderizadas em Chromium
bun run test:e2e                      # exit 0 — navegação por teclado, layout 1200×800 e 900×600 preservando rascunho entre abas (case 10)
python3 scripts/docs-check            # exit 0 — integridade CASA
```

## Human review

- Abrir o app desktop (`bun run dev`) no Linux contra os arquivos reais em `~/.claude`, `~/.codex` e
  `~/.gemini/config` e validar leitura, mascaramento e salvamento com diff.
- Validar navegação completa por teclado, foco visível e contraste em `1200 × 800` e `900 × 600`.
- Conferir que os backups são criados apenas no diretório XDG de dados do app e nenhum `.bak`
  aparece nas pastas nativas.

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
