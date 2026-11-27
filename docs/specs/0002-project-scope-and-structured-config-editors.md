---
status: accepted
date: 2026-11-26
builds-on: [ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0005]
implemented-by: []
design-ref: docs/UI-PLAN.md
---

# Gerenciar configurações por projeto e editores estruturados de MCPs e Settings

## Goal

Expandir o `agents-manager` para suportar o escopo de **Projetos registrados** (armazenados
exclusivamente em XDG sem poluir repositórios) e oferecer **editores estruturados** para **MCPs** e
**Settings** (com alternância `Structured` ↔ `Raw`), utilizando escrita cirúrgica (`toml_edit` para
TOML e merge estrutural preservando chaves desconhecidas para JSON) sobre o mesmo pipeline seguro de
diff, backup rotacionado em XDG e detecção de conflitos da `SPEC-0001`.

## Flow

1. O usuário registra pastas de projeto pela navegação de escopo (`Global` / `Projects` + botão
   `Add project`), persistidas no registro XDG (`projects.json`).
2. Selecionar um projeto exibe os agentes e configurações descobertas no escopo `Project` (`MCPs`,
   `Skills`, `Rules`, `Settings`), sem nunca gravar metadados nem `.bak` dentro do repositório.
3. Ao abrir um arquivo de **MCPs** ou **Settings** (tanto em escopo `Global` quanto `Project`), a UI
   oferece alternância entre o modo **`Structured`** (formulário de servidores MCP com
   `command`/`args`/`url`/`env` e controles de Settings) e o modo **`Raw`** (CodeMirror da
   `SPEC-0001`).
4. Campos de `env` com prováveis segredos literais ficam mascarados por padrão com toggle `Reveal`,
   exibindo dica não-bloqueante recomendando o uso de `${VAR}`.
5. Antes de gravar alterações feitas no modo `Structured`, o frontend chama
   `preview_structured_save(request)`, o backend gera o conteúdo serializado cirurgicamente
   preservando comentários TOML e chaves desconhecidas, e a UI apresenta o **Diff** contra o arquivo
   atual em disco.
6. Confirmar `Save` executa `save_structured_config(request)` passando pelo mesmo pipeline atômico,
   rotação de até 20 backups em XDG e checagem de `expectedSha256` e conflitos externos (`notify`).

## Contract

### Paths por projeto descobertos pelos adapters (`project_root: &Path` injetável)

| Agente          | Categoria       | Caminho(s) relativo(s) ao projeto                        | Formato    |
| --------------- | --------------- | -------------------------------------------------------- | ---------- |
| **Claude Code** | MCPs            | `.mcp.json`                                              | `json`     |
| **Claude Code** | Settings        | `.claude/settings.json`, `.claude/settings.local.json`   | `json`     |
| **Claude Code** | Rules           | `CLAUDE.md`, `CLAUDE.local.md`                           | `markdown` |
| **Claude Code** | Skills          | `.claude/skills/*/SKILL.md`                              | `markdown` |
| **Codex**       | Settings & MCPs | `.codex/config.toml`                                     | `toml`     |
| **Codex**       | Rules           | `AGENTS.md`, `AGENTS.override.md`                        | `markdown` |
| **Codex**       | Skills          | `.agents/skills/*/SKILL.md`, `.codex/skills/*/SKILL.md`  | `markdown` |
| **Antigravity** | Settings        | `.gemini/settings.json`                                  | `json`     |
| **Antigravity** | MCPs            | `.gemini/mcp_config.json`                                | `json`     |
| **Antigravity** | Rules           | `AGENTS.md`, `.gemini/GEMINI.md`, `.agents/rules/*.md`   | `markdown` |
| **Antigravity** | Skills          | `.agents/skills/*/SKILL.md`, `.gemini/skills/*/SKILL.md` | `markdown` |

### Persistência do registro de projetos (XDG)

- Arquivo `projects.json` sob `ProjectDirs` (`directories`), contendo registros
  `{ id, name, rootPath, addedAt }`.
- Remoção ou falha de registro em `projects.json` nunca cria nem altera arquivos dentro do
  `rootPath` do projeto.

### Tipos de domínio (`crates/core`) e IPC (`#[specta::specta]` → `src/bindings/tauri.ts`)

- `ProjectEntry`: `{ id: string, name: string, rootPath: string, addedAt: string }`
- `ProjectSummary`:
  `{ id: string, name: string, rootPath: string, addedAt: string, available: boolean }`
- `McpTransport`: `"stdio" | "http"`
- `McpEnvVar`: `{ key: string, value: string, isSecret: boolean, isEnvRef: boolean }`
- `McpServerConfig`:
  `{ name: string, transport: McpTransport, command?: string, args?: string[], url?: string, env: McpEnvVar[], enabled: boolean }`
- `StructuredSaveRequest`:
  `{ path: string, expectedSha256: string, mcpServers?: McpServerConfig[], settingsPatch?: Record<string, unknown>, forceOverwrite?: boolean }`
- `StructuredPreviewResult`: `{ content: string, secretSpans: SecretSpan[] }`

### Commands Tauri

- `list_projects() -> Result<Vec<ProjectSummary>, CoreError>`
- `register_project(root_path: String) -> Result<ProjectSummary, CoreError>`
- `remove_project(project_id: String) -> Result<(), CoreError>`
- `list_project_configs(project_id: String, kind: AgentKind) -> Result<Vec<ConfigFileEntry>, CoreError>`
- `preview_structured_save(request: StructuredSaveRequest) -> Result<StructuredPreviewResult, CoreError>`
- `save_structured_config(request: StructuredSaveRequest) -> Result<SaveConfigResult, CoreError>`

### Garantias de escrita cirúrgica e allowlist

- **TOML cirúrgico:** Edições em `.codex/config.toml` ou `~/.codex/config.toml` usam `toml_edit`
  para atualizar apenas os campos editados (ex.: tabela `[mcp_servers.<name>]`), preservando
  comentários, ordenação e chaves desconhecidas.
- **JSON cirúrgico:** Edições em `.mcp.json`, `mcp_config.json` e `settings.json` executam merge
  estrutural preservando chaves desconhecidas no objeto raiz e nos objetos de servidor.
- **Allowlist estendida:** Leitura, escrita e watching aceitam apenas caminhos descobertos pelos
  adapters no escopo global ou dentro de projetos atualmente registrados em `projects.json`.

## Edge cases

| #   | WHEN ⟨trigger⟩                                                                                            | THE SYSTEM SHALL ⟨response⟩                                                                                                                            |
| --- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | The user submits a nonexistent path or a file path to `register_project`                                  | Reject with a typed validation error without modifying `projects.json`                                                                                 |
| 2   | The user registers a directory whose canonical path is already in `projects.json`                         | Prevent duplicate entries and show clear feedback pointing to the existing project                                                                     |
| 3   | A registered project directory is moved or deleted externally                                             | List the project with status `Unavailable` and a `Remove from list` action without breaking other projects                                             |
| 4   | The user removes a project via `remove_project`                                                           | Delete only the entry from XDG `projects.json` without touching any file in the repository                                                             |
| 5   | The user saves structured MCP edits to a `config.toml` containing comments and unknown keys               | Use `toml_edit` to update only the edited tables/fields while preserving all comments and unknown keys                                                 |
| 6   | The user saves structured edits to a JSON config (`.mcp.json` or `settings.json`) containing unknown keys | Merge only the edited fields and preserve all unknown keys in the output JSON                                                                          |
| 7   | The user switches between `Structured` and `Raw` modes with unsaved changes                               | Preserve the active draft across mode switches; if `Raw` contains syntax errors, block switching back to `Structured` with an inline error until fixed |
| 8   | A literal secret value is entered into an MCP `env` field in `Structured` mode                            | Mask the value by default with a `Reveal` toggle and show a non-blocking hint recommending `${VAR}`                                                    |
| 9   | Disk content diverges from `expectedSha256` or `notify` fires during structured editing                   | Return `CoreError::ExternalConflict` on save and surface explicit `Reload from disk` / `Overwrite (review diff)` choices                               |
| 10  | A registered project contains no agent configuration files yet                                            | Render a clear empty state per agent/category and never create `.bak` files or metadata inside the project repository                                  |

## Out of scope

- **`SPEC-0003` — Biblioteca Compartilhada Opcional e Sincronização:** promoção de itens para
  biblioteca XDG, symlinks com fallback explícito para cópia em Skills/Regras, merge estruturado
  JSON ↔ TOML entre agentes e preview de diff de sync.
- **Fora da v1:** suporte a macOS e Windows, tradução/i18n da UI para pt-BR, TanStack MCP e testes
  de regressão visual por screenshots.

## Open questions

Nenhum item em aberto (decisões fechadas no brainstorm).

## Definition of Done

```bash
cargo test -p agents-manager-core     # exit 0 — registro de projetos em XDG, canonicalização, toml_edit preservando comentários, merge cirúrgico JSON e conflito estruturado (cases 1, 2, 4, 5, 6, 9)
cargo test -p agents-manager-adapters # exit 0 — descoberta de paths por projeto para Claude Code, Codex e Antigravity + allowlist estendida (cases 3, 10)
bun run check                         # exit 0 — Biome + tsc --noEmit + cargo fmt --check + cargo clippy --workspace
bun run check:guards                  # exit 0 — boundaries, tamanho de componente, style drift, stories, TanStack e design system
bun run test:ts                       # exit 0 — alternância Structured/Raw, mascaramento em env com dica ${VAR}, estados Unavailable/Empty e conflito (cases 3, 7, 8, 9, 10)
bun run test:storybook                # exit 0 — stories renderizadas em Chromium
bun run test:e2e                      # exit 0 — fluxo de registro de projeto, editor estruturado com diff e navegação por teclado
python3 scripts/docs-check            # exit 0 — integridade CASA
```

## Human review

- Registrar um repositório Git real no app desktop (`bun run dev`), editar MCPs e Settings no modo
  `Structured`, revisar o diff gerado, salvar e confirmar que `git status` no repositório alvo
  continua limpo de `.bak` ou metadados internos.
- Validar manualmente a edição de um `config.toml` com comentários e chaves customizadas garantindo
  que nenhum comentário foi perdido.
- Conferir navegação por teclado nos formulários de MCPs e Settings em `1200 × 800` e `900 × 600`.

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
