---
status: proposed
date: 2026-11-26
builds-on: [ADR-0001, ADR-0002]
superseded-by: null
deciders: [gugacarbo]
---

# Adotar arquitetura de adapters extensíveis com paths injetáveis

## Context and Problem

Cada ferramenta de IA possui convenções próprias de diretórios, nomes de arquivos e formatos para
MCPs, Skills, Regras e Settings, além de variações entre Linux, macOS e Windows. Misturar caminhos
fixos de `$HOME` e regras específicas de cada ferramenta dentro dos commands Tauri ou do core
impediria testar com segurança e dificultaria adicionar novos agentes no futuro.

## Decision Drivers

- Suportar Claude Code, Codex e Antigravity na v1 cobrindo MCPs, Skills, Regras/Instruções e
  Settings em escopo global e por projeto.
- Manter o núcleo pronto para futura distribuição open-source e novos adapters (ex.: Cursor,
  Windsurf, OpenCode) sem reescrever a UI ou o motor de sync.
- Garantir testes unitários e de integração determinísticos sem ler ou modificar o `$HOME` real do
  desenvolvedor.
- Focar a validação da v1 em Linux sem acoplar o design a premissas exclusivas de POSIX onde a
  camada de path pode abstrair.

## Considered Options

### Option 1 — Trait AgentAdapter desacoplada com resolução de paths injetável (Linux-first, cross-platform por design)

**Pros:** Isola descoberta, leitura e serialização específica de cada agente em `crates/adapters`
atrás de contratos explícitos (`AgentAdapter`), injeta diretórios base (`home: &Path`,
`project_root: &Path`) para testes isolados com `tempfile`, e concentra a resolução de paths do SO
na fronteira de runtime (`directories`).

**Cons:** Validação prática de empacotamento e casos de borda em macOS e Windows fica adiada para
além da v1.

### Option 2 — Lógica condicional por agente diretamente no core ou nos commands Tauri

**Pros:** Menor abstração inicial para apenas três ferramentas.

**Cons:** Cria acoplamento direto entre UI/IPC e formatos específicos, dificulta testes isolados e
torna cada novo agente uma mudança espalhada pelo projeto.

### Option 3 — Suporte e validação completa Linux/macOS/Windows desde a v1

**Pros:** Distribuição imediata para os três sistemas operacionais.

**Cons:** Multiplica o custo de validação na v1 com tratamento de junctions/symlinks no Windows,
diferenças de shell/env em MCPs e empacotamento em três plataformas.

## Decision

Adotar uma **arquitetura de adapters extensíveis** em `crates/adapters` baseada em traits
(`AgentAdapter`) com diretórios base injetáveis (`&Path`), mantendo a v1 validada em **Linux** e
**cross-platform por design** nos tipos e na resolução de paths.

O escopo inicial da v1 cobre **Claude Code**, **Codex** e **Antigravity** nos quatro pilares
(**MCPs, Skills, Regras/Instruções e Settings**) em escopo global e de projetos registrados,
deixando novos adapters e empacotamento macOS/Windows para etapas posteriores.

## Consequences

- **Positivas:** Novos agentes entram como novos módulos implementando os contratos de
  `crates/adapters`; testes rodam isolados em diretórios temporários.
- **Negativas:** Funcionalidades que dependem de comportamento específico de sistema operacional
  (ex.: symlinks no Windows) exigirão validação dedicada ao expandir de plataforma.
- **Obrigatório:** Adapters em `crates/adapters` devem receber caminhos base por parâmetro (`&Path`)
  e nunca consultar variáveis globais de ambiente ou `$HOME` diretamente dentro da lógica de
  parsing/detecção.
- **Proibido:** Fixar lógica específica de um agente na camada de UI (`src/`) ou no crate de domínio
  puro (`crates/core`).

## Confirmation

```bash
cargo test -p agents-manager-adapters
```

## Notes

- Implementação inicial em `crates/adapters/src/lib.rs`, `claude_code.rs`, `codex.rs` e
  `antigravity.rs`.
