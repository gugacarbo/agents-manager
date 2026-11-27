# AGENTS.md

```yaml
casa-repo-id: agents-manager # used in cross-repo references (repo:ADR-0001)
casa-tier: T1 # T0 (light) | T1 (standard) — STANDARD §3
casa-version: 1.11 # adopted CASA contract version (repo promise, ADR-0010)
casa-standard-ref: d28335d # source casa-standard snapshot — stamped by casa-init
```

> Standard: https://github.com/atplus-digital/casa-standard (STANDARD.md) ROUTER (CASA §4): always
> loaded, ~150-line cap. Cross-cutting high-ROI only. Over the cap → push details down to
> docs/context/, keep only a pointer here. ⚠️ DO NOT use @import to inline chapters: @import expands
> everything at launch. Package-specific rules → <subdir>/AGENTS.md (native lazy loading,
> nearest-wins).

## Context in 5 lines

App desktop local em **Tauri v2** (core em Rust via Cargo Workspace + UI React/Vite/Tailwind/Bun)
para centralizar a visualização, edição in-place e sincronização opcional (via biblioteca
compartilhada) de configurações de agentes de IA (**Claude Code**, **Codex** e **Antigravity** na
v1), cobrindo MCPs, Skills, Regras e Settings em escopo global e por projeto. Uso pessoal
Linux-first na v1, com arquitetura de adapters desacoplada para futura distribuição open-source.

## Infra & environments

- Execução 100% local no desktop (sem servidor web, Docker, PostgreSQL ou serviços remotos).
- Use **Bun** (`bun install`, `bun run ...`, `bunx ...`); nunca use `npm`, `pnpm` ou `yarn`, nem
  crie `package-lock.json`.
- No Linux (Ubuntu/Debian), a compilação de `src-tauri` exige as libs nativas de sistema
  (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`, `libssl-dev`, `librsvg2-dev`,
  `libayatana-appindicator3-dev`, `libxdo-dev`).

## How to run locally

```bash
bun install
bun run dev       # app desktop Tauri v2 + Vite com hot-reload
bun run dev:web   # frontend Vite isolado (http://localhost:1420)
bun run storybook # catálogo de componentes Storybook (http://localhost:6006)
```

## How to validate (global repo DoD)

```bash
python3 scripts/docs-check # exit 0 — CASA gate (STANDARD §8)
bun run check              # exit 0 — Biome + tsc --noEmit + cargo fmt --check + cargo clippy
bun run format:check       # exit 0 — Prettier em md/yml/yaml/sh
bun run spell              # exit 0 — CSpell (en, pt_BR)
bun run knip               # exit 0 — detecção de código/deps não usados
bun run test               # exit 0 — Vitest + cargo test --workspace em paralelo
bun run build:web          # exit 0 — Vite production build
bun run build-storybook    # exit 0 — Storybook static build
bun run test:e2e           # exit 0 — Playwright E2E (Chromium)
```

## How to deploy

- Empacotamento desktop local completo via `bun run build` (`tauri build`; gera binário/pacotes em
  `src-tauri/target/release/bundle/`).
- Sem deploy remoto ou distribuição automática na v1.

## Git & PRs

- Commits seguem **Conventional Commits** validados por Commitlint (`.husky/commit-msg`).
- O hook `.husky/pre-commit` executa Gitleaks (`--staged`), `python3 scripts/docs-check` e
  `lint-staged`.
- Não execute `git commit`, `git push` nem abra PRs sem solicitação explícita.

## Gotchas

- `src/bindings/**` é gerado automaticamente por `tauri-specta` a partir do Rust — não edite
  manualmente (ignorado por Biome, Knip e CSpell).
- Nunca adicione `fixtures/**` ou `tests/**` na allowlist global do `.gitleaks.toml` (risco de vazar
  credenciais copiadas de configs reais).
- Clippy do workspace proíbe `unwrap_used`, `expect_used`, `dbg_macro` e `print_stdout` em produção;
  use `#[allow(...)]` apenas pontualmente se alguma macro do Tauri/Specta exigir.

## Context map

<!-- Index of chapters (docs/context/), each stating WHEN to load it.
     Chapter = current, imperative, timeless state. Dated decision = ADR. -->

| Chapter    | When to load |
| ---------- | ------------ |
| (none yet) | —            |

## Docs map

- Decisions: `docs/adr/` · Behavior: `docs/specs/` (GENERATED READMEs — do not edit)
- Validate: `scripts/docs-check` · Regenerate indices: `scripts/docs-check --emit-index`
