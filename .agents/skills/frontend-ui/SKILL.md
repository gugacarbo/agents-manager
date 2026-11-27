---
name: frontend-ui
description:
  Build and modify frontend UI components, patterns, and feature views using the repository
  component catalog, semantic design tokens, Storybook stories, and architectural guardrails.
---

# Frontend UI Skill

## Required First Step

Before creating or modifying any component in `src/components/` or `src/features/`:

1. Read `.agents/skills/frontend-ui/references/components.generated.md` to inspect every existing
   pattern, product component, and shadcn/ui primitive.
2. If you added, renamed, or removed components, regenerate the catalog with:
   `bun scripts/generate-ui-catalog.ts`.

## Component Composition Hierarchy

Always compose from the highest applicable layer down, creating a new component only when no higher
layer covers the need:

`Layout/Route -> Pattern -> Product Component -> shadcn Primitive -> Novo componente`

- **Layout/Route (`src/router.tsx`, `src/features/<name>/*Page.tsx`)**: Wire data hooks and arrange
  patterns/components. Keep files at or below 250 lines.
- **Pattern (`src/components/patterns/*.tsx`)**: Reusable multi-part UI compositions (toolbars,
  empty/error states, split views). Must include a sibling `.stories.tsx`.
- **Product Component (`src/components/*.tsx`)**: Domain-specific reusable blocks (cards, editors,
  badges). Must include a sibling `.stories.tsx`.
- **shadcn Primitive (`src/components/ui/*.tsx`)**: Low-level accessible UI primitives (`Button`,
  `Badge`, `Card`, `Resizable`, `CommandPalette`, `Toaster`).

## Styling & Design Tokens

- Use semantic tokens defined in `src/index.css` (`bg-background`, `text-foreground`, `bg-card`,
  `border-border`, `bg-primary`, `text-destructive`, `text-muted-foreground`, etc.).
- In `src/features/**` and `src/router.tsx`:
  - Never use inline `style={{ ... }}`.
  - Never use arbitrary Tailwind brackets (`w-[...]`, `h-[...]`, `text-[...]`, `p-[...]`, etc.).
  - Never use raw `<button>` elements; always use `Button` from `@/components/ui/button`.

## Stories, Boundaries & Validation

- Every new `.tsx` file in `src/components/` or `src/components/patterns/` (except `ui/`,
  `index.ts`, `theme-provider.tsx`, `*.test.*`, `*.stories.*`) requires a sibling `.stories.tsx`.
- Respect feature boundaries: import features only through `@/features/<name>` (`index.ts`).
- Never import `@tauri-apps/api/core` inside `src/components/**`, `src/features/**`, or
  `src/router.tsx`; route IPC through `src/bindings/` or `src/lib/`.
- Run `bun scripts/validate.ts guards` after UI edits to verify all guardrails pass.
