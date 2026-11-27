# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

The documented v1 operating intent is personal use, Linux-first.

**Documentary inference — user confirmation pending:** The primary user is a developer who uses one
or more of Claude Code, Codex, and Antigravity and needs to locate and understand their scattered
local configuration. Their current job is to see which supported agents are present and where their
configuration directories live. Their broader proposed job is to manage selected MCPs, Skills,
Rules, and Settings across global and project scopes without manually duplicating them; this broader
workflow is not implemented.

No additional audience, team use case, or customer segment has been confirmed.

## Product Purpose

The current product detects supported local AI coding agents and presents their detection status and
configuration paths, including loading and error states.

**Documentary inference — user confirmation pending:** Agents Manager is intended to become a single
local control point for inspecting, editing, and selectively sharing AI agent configuration while
preserving each agent's native files and formats. This purpose is derived from `AGENTS.md` and
proposed ADRs; only agent detection and inventory are currently connected to the backend.

**Open decision — no user response:** Success criteria are undefined. No adoption target,
task-completion measure, reliability threshold, performance benchmark, or release criterion has been
established. Repository validation commands are engineering checks, not evidence of product success.

## Positioning

**Documentary inference — user confirmation pending:** The proposed position is a local-first
manager that works with existing Claude Code, Codex, and Antigravity configuration in place, with an
optional shared library for deliberate reuse rather than mandatory migration. The proposed
distinction is the combination of native global/project files, explicit synchronization, and
agent-specific adapters.

This is proposed product direction, not a verified competitive claim or a description of shipped
editing and synchronization capabilities. No competitor research or positioning validation is on
hand.

## Operating Context

- The product is a local desktop application using a Tauri v2 shell and a web UI; its `web` platform
  classification does not make it a hosted web service.
- The documented v1 environment is personal and Linux-first. Linux is the initial validation target;
  support for other operating systems is deferred.
- There is no application server, remote database, Docker requirement, or external daemon in the
  documented architecture.
- The current UI runs inside a WebView whose configured capability permission is `core:default`;
  real agent detection is routed through the Rust backend.
- The three detection locations are `~/.claude`, `~/.codex`, and `~/.gemini/antigravity`.
- Local desktop packaging is available through `bun run build`. `AGENTS.md` states that v1 has no
  remote deployment or automatic distribution.

## Capabilities and Constraints

### Implemented

- One registered IPC operation, `list_agents`, returns summaries for Claude Code, Codex, and
  Antigravity.
- Adapters detect the three native configuration directories and report detection status and
  configuration paths.
- The overview presents the supported agents and their paths, with loading and error states and
  client-side inventory filtering.
- The standalone web development runtime returns fallback summaries rather than detecting the real
  local machine.

### Demonstration only, not connected product capability

- The editor/diff route operates on sample JSON held in frontend state. Its own UI states that
  backend sync comes next; it does not read, edit, save, or synchronize real agent configuration.

### Proposed plans, not implemented capabilities or confirmed requirements

- Proposed ADR-0002 describes global/project in-place editing, an optional shared library, mandatory
  diff preview for synchronization, symlinks with explicit copy fallback for Skills/Rules, and
  structured JSON/TOML merge for MCPs/Settings.
- Proposed ADR-0001 and ADR-0004 describe structured editors, search, conflict handling, backups,
  atomic writes, preservation of file structure, and secret masking.
- Proposed ADR-0005 describes XDG-based local storage and retention of up to 20 backup snapshots per
  target file.
- These ADRs have `status: proposed`. They must not be presented as shipped behavior, validated
  safeguards, or user-approved scope.

### Open product decisions

- **Distribution — no user response:** The repository documents local packaging, personal-use v1,
  and possible future open-source distribution, but no distribution channel, release cadence, update
  mechanism, or commitment to publish has been decided. The README declares an MIT license; that
  alone does not settle distribution.
- **Language — no user response:** Current UI copy is English, but the required product language or
  languages and any localization requirements are undecided.
- **Scope confirmation — no user response:** The proposed editing, sharing, safety, and backup
  workflows require user confirmation before being treated as committed product scope.

## Brand Commitments

The current working name is **Agents Manager**, as used in the README and UI. No product identity or
brand direction has been confirmed by the user.

## Evidence on Hand

- `README.md` and `AGENTS.md`: product summary, local desktop architecture, intended agents,
  personal Linux-first v1, local packaging, validation commands, and MIT license declaration.
- `docs/adr/0001-tauri-v2-rust-workspace-and-specta-ipc.md`,
  `docs/adr/0002-hybrid-in-place-and-shared-library-model.md`,
  `docs/adr/0004-safe-surgical-writes-conflict-detection-and-secrets.md`, and
  `docs/adr/0005-xdg-file-storage-library-layout-and-backup-retention.md`: proposed architecture and
  product direction; all are marked `proposed`.
- `src/App.tsx`, `src/router.tsx`, and `src/lib/agents.ts`: current routing, overview behavior,
  sample-only editor/diff demonstration, and frontend IPC handling.
- `crates/adapters/src/` and `src-tauri/src/commands/mod.rs`: implemented detection paths and the
  single agent-listing command.
- `src/App.test.tsx` and `tests/e2e/app.spec.ts`: schema, rendering, error, and role-based UI test
  coverage.

There is no user interview response, customer evidence, usage data, benchmark result, usability
study, or validated product metric on hand. Future work must not invent them.

## Product Principles

All principles below are **documentary inferences — user confirmation pending**. They describe
proposed strategic direction, not implemented guarantees.

1. **Meet the existing setup where it is.** Prefer working with native configuration over forced
   migration; make any shared-library adoption optional and incremental.
2. **Make consequential changes explicit.** Proposed writes and synchronization should require
   reviewable diffs, surface external conflicts, and provide recovery through backups before
   claiming safety.
3. **Respect agent differences.** Preserve native formats, scopes, comments, and unknown fields;
   isolate agent-specific behavior behind adapters rather than pretending configurations are
   interchangeable.
4. **Keep control local and exposure deliberate.** Avoid service dependencies for the core workflow,
   and treat secret masking and copy warnings as proposed requirements until implemented and tested.

## Accessibility & Inclusion

**Open decision — no user response:** No required accessibility standard, WCAG version, or
conformance level has been selected.

The current UI contains landmarks, roles, and `aria-label` attributes, and tests query some elements
by role. These are implementation signals only. No evidence of axe checks, contrast validation,
complete keyboard testing, or screen-reader testing was found; accessibility conformance must not be
claimed.
