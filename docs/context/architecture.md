# Arquitetura

Visão atual das fronteiras arquiteturais do `agents-manager`. Decisões históricas e justificativas
permanecem nas ADRs em `docs/adr/`.

## Cargo Workspace em Rust

O `Cargo.toml` raiz define um workspace Rust (edition 2024) com cinco membros e lints globais
estritos (`unsafe_code = "forbid"`, `unwrap_used = "deny"`, `expect_used = "deny"`,
`dbg_macro = "deny"`, `print_stdout = "deny"`):

- `crates/core` (`agents-manager-core`): concentra tipos de domínio compartilhados no IPC
  (`domain.rs`), erros tipados (`CoreError` em `error.rs`), leitura/escrita atômica com SHA-256,
  backup rotacionado e detecção de segredos (`config.rs`), edição estrutural JSON/TOML
  (`structured.rs`), registro de projetos (`projects.rs`), biblioteca compartilhada e links de
  sincronização (`library.rs`) e preferências (`preferences.rs`), mantendo o núcleo agnóstico a
  sistema operacional.
- `crates/adapters` (`agents-manager-adapters`): implementa a trait `AgentAdapter` (`Send + Sync`)
  para **Claude Code**, **Codex** e **Antigravity** (`claude_code.rs`, `codex.rs`,
  `antigravity.rs`), além de `resolve_sync_target`. Adapters recebem diretórios base por parâmetro
  (`&Path`) e nunca leem `$HOME` diretamente.
- `crates/remote` (`agents-manager-remote`): encapsula o registro de ambientes e o transporte
  OpenSSH sem shell (`ssh.rs`), bem como a edição remota atômica restrita a alvos fechados
  (`remote_config.rs`, `remote_config.py`), reutilizando DTOs e `CoreError` de `crates/core`.
- `crates/claude-accounts` (`agents-manager-claude-accounts`): agrupa o isolamento de perfis da CLI
  do Claude Code no Linux (`accounts.rs`, `accounts/process.rs`) e a coleta de cotas por perfil
  isolado do Claude Desktop (`desktop_quotas.rs`, `desktop_quotas/source.rs`), isolando dependências
  de processo, criptografia local e HTTP do núcleo.
- `src-tauri`: mantém o shell desktop **Tauri v2**, handlers IPC em `src-tauri/src/commands/`,
  estado em runtime (`watcher.rs`, `sync_preview.rs`, `remote_state.rs`, `preferences_state.rs`),
  bandeja do sistema (`tray.rs`), resolução XDG via `directories` e capabilities restritas em
  `src-tauri/capabilities/default.json`.

## Fronteira IPC Tauri v2 e `tauri-specta`

- Exponha comandos do backend exclusivamente em `src-tauri/src/commands/` com `#[tauri::command]` e
  `#[specta::specta]`, registrados no `specta_builder` em `src-tauri/src/lib.rs`.
- Nunca edite `src/bindings/tauri.ts` manualmente: o arquivo é gerado por `tauri-specta` e validado
  por testes em `src-tauri/src/lib.rs` e `src-tauri/src/ipc_smoke.rs`.
- O WebView opera sem permissões amplas de filesystem ou shell; toda operação em arquivos ou
  processos externos atravessa comandos tipados no backend Rust.

## Estrutura do Frontend (`src/`)

- `src/main.tsx` configura `QueryClientProvider` (`@tanstack/react-query`) e `src/router.tsx`
  declara as rotas via `@tanstack/react-router` com `createHashHistory`.
- `src/lib/` é a fronteira única do frontend com o IPC: encapsula chamadas de
  `src/bindings/tauri.ts`, valida retornos com `zod`, checa o runtime desktop (`isTauriRuntime()`),
  normaliza erros de IPC para a UI e isola utilitários puros (validação estruturada, segredos no
  editor, tema).
- `src/features/*` organiza cada domínio de tela (`agents`, `claude-accounts`, `configs`, `diff`,
  `environments`, `library`, `projects`, `rules`) atrás de barrel exports em `index.ts`. Importar
  arquivos internos de outra feature é proibido.
- `src/components/` reúne componentes compartilhados, padrões em `src/components/patterns/`,
  primitivas em `src/components/ui/` e tokens semânticos em `src/index.css`.

## Subsistema SSH Remoto

- O cadastro de ambientes persiste apenas metadados sob XDG via `crates/remote/src/ssh.rs`, sem
  alterar a configuração do OpenSSH do usuário.
- `SshTransport` executa o cliente `ssh` do sistema sem shell local e com flags não interativas
  estritas (`BatchMode=yes`, `StrictHostKeyChecking=yes`, `ForwardAgent=no`,
  `ClearAllForwardings=yes`, `PermitLocalCommand=no`).
- `crates/remote/src/remote_config.rs` restringe operações a um conjunto fechado de alvos e executa
  `remote_config.py` com entrada JSON, validando identidade do arquivo e substituição atômica sem
  seguir links simbólicos.
- `src-tauri/src/remote_state.rs` (`RemoteState`, `ReviewVault`) e
  `src-tauri/src/commands/remote.rs` mantêm sessões abertas e tokens de revisão no backend,
  vinculando cada escrita à revisão confirmada e sem repetição automática em perda de confirmação.

## Contas Claude Code e Perfis de Cota do Claude Desktop (Linux)

- Compilado apenas no Linux, o módulo de contas em `crates/claude-accounts/src/accounts.rs` e
  `crates/claude-accounts/src/accounts/process.rs` cria perfis privados no XDG de dados e injeta
  `CLAUDE_CONFIG_DIR` apenas nos processos filhos da CLI oficial (`claude auth login` e
  `claude auth status`). `is_private_path` bloqueia esses diretórios no editor, na sincronização e
  nos backups.
- `crates/claude-accounts/src/desktop_quotas.rs`,
  `crates/claude-accounts/src/desktop_quotas/source.rs`, `src-tauri/src/claude_desktop_launcher.rs`
  e `src-tauri/src/commands/claude_desktop_quotas.rs` isolam cada perfil do Claude Desktop em
  diretórios XDG privados sob `claude-desktop-quotas`, gerenciam a abertura do login isolado, exigem
  o Desktop fechado durante a leitura, validam a identidade vinculada e mantêm tokens apenas em
  memória durante a coleta.

## Guardrails de Qualidade e Arquitetura

- `scripts/docs-check`: valida `AGENTS.md` (metadados CASA, DoD global, ponteiros em `Context map` e
  limite de 150 linhas), frontmatter, grafo causal, imutabilidade de ADRs aceitas e índices gerados
  (`docs/index.json`, `docs/adr/README.md`, `docs/specs/README.md`).
- `scripts/validate.ts`: executa `guards`, `fast` e `local`. Os seis guardrails em `scripts/`
  cobrem: `check-feature-boundaries.ts` (sem imports profundos/cruzados entre `src/features/*`, sem
  `@tauri-apps/api/core` direto em UI/router e sem imports não-tipo de `src/bindings/` em camadas de
  UI/router), `check-component-size.ts` (máximo de 250 linhas em `.tsx` de `src/components/` e
  `src/features/`, exceto `ui/`, testes e stories), `check-style-drift.ts` (sem `style={{...}}`,
  classes arbitrárias Tailwind ou `<button>` nativo em `src/features/` e `src/router.tsx`),
  `check-story-coverage.ts` (exige `.stories.tsx` irmã em componentes compartilhados),
  `check-tanstack-conventions.ts` (bloqueia APIs depreciadas de Query e exige `tableFeatures(...)`
  no topo do módulo) e `check-design-system.ts` (máximo de 160 linhas e tokens obrigatórios em
  `src/index.css`).
- `bun run check` complementa os guardrails com Biome (`biome.json`: máximo de 400 linhas fora de
  testes e `noConsole`), `tsc --noEmit`, `cargo fmt --all -- --check` e `cargo clippy`.
