---
status: accepted
date: 2026-11-27
builds-on: [ADR-0009, SPEC-0007]
implemented-by: []
---

# Trocar contas do Claude Desktop por perfil isolado

## Goal

Abrir uma conta vinculada do Claude Desktop no Linux pelo Agents Manager, usando a sessão salva no
respectivo perfil.

## Flow

1. Na lista de contas vinculadas, o usuário seleciona a conta de destino.
2. Se o Claude Desktop estiver aberto, o Manager orienta o usuário a fechá-lo antes de continuar.
3. O Manager inicia o mesmo executável com os diretórios do perfil selecionado. Para a conta da
   sessão padrão, usa os diretórios padrão.
4. Se a sessão expirar, o usuário faz login pelo fluxo do próprio perfil e pode tentar novamente.

## Contract

- A seleção resolve o perfil já vinculado, incluindo vínculos existentes. Uma conta pendente
  permanece no fluxo de cadastro da SPEC-0007.
- O Manager inicia uma troca por vez. O login padrão e os outros perfis preservam suas sessões.
- Quando a conta também tiver um perfil isolado confirmado, a linha da conta oferece abertura
  explícita da sessão padrão e do perfil isolado.
- A opção de importar conversas é definida na SPEC-0009.

## Edge cases

| #   | Quando                                                                        | O sistema deve                                                               |
| --- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| 1   | O Desktop ainda estiver aberto                                                | Orientar a fechar o Desktop e não iniciar outra instância.                   |
| 2   | O perfil estiver pendente ou indisponível                                     | Informar o estado e oferecer o fluxo correspondente, sem abrir outro perfil. |
| 3   | O destino for a sessão padrão ou um perfil isolado existente                  | Abrir os diretórios do destino correto e preservar os demais logins.         |
| 4   | Duas trocas forem solicitadas simultaneamente ou o processo falhar ao iniciar | Impedir abertura concorrente, informar a falha e permitir nova tentativa.    |

## Open questions

Nenhuma de produto. A compatibilidade com duas contas reais no Linux é uma condição do DoD.

## Definition of Done

Estender a suíte Rust de perfis e a suíte da tela de contas para exercitar a abertura e seus erros.

```bash
cargo test --workspace --test claude_account_quotas                       # exit 0, estado e vínculos dos casos 2–3
cargo test -p agents-manager --lib commands::claude_desktop_quotas::tests # exit 0, ambiente e bloqueio dos casos 1, 3 e 4
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # exit 0, casos 1–4
python3 scripts/docs-check                                                # exit 0
```

Com `bun run dev`, vincular duas contas reais no Linux pelo fluxo da SPEC-0007. Alternar entre elas
e verificar a identidade no Desktop em cada abertura. Fechar e reabrir o Manager e repetir a troca
sem novo login, verificando também a abertura da sessão padrão.

## Human review

Conferir a identificação do perfil de destino e as instruções quando o Desktop está aberto.
