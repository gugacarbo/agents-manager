## Resumo

- Descreva a mudança e o problema resolvido.
- Referencie ADR, Spec ou issue relacionada (se houver).

## Validações

- [ ] `python3 scripts/docs-check`
- [ ] `bun run check` (`biome`, `tsc`, `cargo fmt`, `cargo clippy`)
- [ ] `bun run test` (`vitest` e `cargo test --workspace`)
- [ ] `bun run format:check` e `bun run spell`
- [ ] `bun run knip`
- [ ] `bun run build:web` / `bun run build-storybook` / `bun run test:e2e` (quando aplicável)

## Revisão visual (UI / Storybook)

- Não aplicável (sem alterações de interface) ou:
- [ ] Validado no Storybook (`bun run storybook`) nos estados vazio, carregamento, erro e sucesso
- [ ] Validado em janela `1200×800` e `900×600`, navegação por teclado e foco visível
- [ ] Capturas ou notas visuais anexadas (Tauri WebView / browser)

## Alternativas avaliadas

- Liste brevemente outras abordagens consideradas e por que foram descartadas (ou "Nenhuma").
