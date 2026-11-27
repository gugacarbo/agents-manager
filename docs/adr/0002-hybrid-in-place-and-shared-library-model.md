---
status: accepted
date: 2026-11-26
builds-on: [ADR-0001]
superseded-by: null
deciders: [gugacarbo]
---

# Adotar modelo híbrido de edição in-place e biblioteca compartilhada opcional

## Context and Problem

Configurações de agentes de IA (MCPs, Skills, Regras/Instruções e Settings) ficam espalhadas em
diretórios e formatos heterogêneos (JSON, TOML, Markdown) tanto no escopo global quanto dentro de
repositórios de projeto. Exigir migração prévia do usuário cria atrito inicial e pode conflitar com
agentes que atualizam seus próprios arquivos em runtime, enquanto editar apenas isoladamente não
resolve a duplicação entre agentes.

## Decision Drivers

- Valor imediato ao abrir o aplicativo sobre um ambiente já configurado, sem migração destrutiva.
- Reutilização prática de MCPs, Skills e Regras entre Claude Code, Codex e Antigravity.
- Compatibilidade com arquivos nativos em diferentes formatos (JSON ↔ TOML) e suporte a escopo
  global e por projeto.
- Controle explícito do usuário antes de qualquer propagação ou alteração em massa.

## Considered Options

### Option 1 — Modelo híbrido (edição in-place + biblioteca compartilhada opcional)

**Pros:** Lê e edita arquivos nativos diretamente no disco desde o primeiro acesso e permite
promover ou criar itens em uma biblioteca compartilhada opcional para sincronizar entre agentes via
symlink (com fallback explícito para cópia) em Skills/Regras e merge/tradução de formato (JSON ↔
TOML) em MCPs/Settings.

**Cons:** Exige lidar explicitamente com conflitos quando um arquivo nativo sincronizado é alterado
externamente por outro editor ou pelo próprio agente.

### Option 2 — Somente edição in-place (sem biblioteca compartilhada)

**Pros:** Arquitetura mais simples e sem estado próprio de sincronização.

**Cons:** Mantém a dor de copiar e atualizar manualmente o mesmo MCP ou Skill em cada ferramenta.

### Option 3 — Fonte única de verdade obrigatória (Hub & Sync)

**Pros:** Reprodutibilidade determinística de todo o setup em um único diretório central.

**Cons:** Exige importar e adotar todas as configurações antes do primeiro uso e aumenta o risco de
sobrescrever alterações feitas pelos próprios agentes em runtime.

## Decision

Adotar o **modelo híbrido**: operar nativamente in-place sobre as configurações globais e por
projeto de cada agente e oferecer uma **biblioteca compartilhada opcional** para sincronização
controlada entre agentes.

Na sincronização a partir da biblioteca compartilhada, Skills e Regras utilizam symlinks (com
fallback explícito para cópia quando necessário), enquanto MCPs e Settings utilizam merge
estruturado com tradução entre formatos nativos (JSON ↔ TOML) e preview obrigatório de diff antes da
aplicação.

## Consequences

- **Positivas:** Zero atrito de adoção inicial; permite migração gradual apenas dos itens que o
  usuário deseja compartilhar entre agentes.
- **Negativas:** Requer rastreamento de origem/destino e detecção de divergência entre biblioteca e
  arquivos nativos.
- **Obrigatório:** Toda operação de sync entre biblioteca compartilhada e agentes deve apresentar
  preview de diff antes de gravar no disco.
- **Proibido:** Exigir importação prévia ou assumir controle exclusivo sobre todos os arquivos de
  configuração dos agentes.

## Confirmation

```bash
rg -n "ConfigScope" crates/core/src/domain.rs && cargo test -p agents-manager-core
```

## Notes

- Escopos `Global` e `Project` definidos em `crates/core/src/domain.rs`.
