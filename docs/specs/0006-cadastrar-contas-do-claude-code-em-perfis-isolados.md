---
status: accepted
date: 2026-11-27
builds-on: [ADR-0008]
implemented-by: []
---

# Cadastrar contas do Claude Code em perfis isolados

## Goal

Cadastrar contas do Claude Code no desktop Linux mediante login oficial por perfil, sem alterar o
login padrão do Claude Code usado fora do aplicativo. O cadastro funciona sem monitoramento de
cotas.

## Flow

1. Na tela de contas, o usuário adiciona um perfil com um nome de exibição.
2. O usuário inicia o login daquele perfil e conclui a autenticação na experiência oficial da CLI,
   fora dos formulários do aplicativo.
3. O app confirma o estado pelo status da CLI no mesmo perfil e o mostra na lista. Reabrir o app
   mantém o cadastro; o estado de conexão não é presumido a partir do cadastro salvo.

## Contract

- Um cadastro corresponde a um perfil estável e independente. A lista distingue login em andamento,
  conectado, desconectado e falha de verificação; só confirma conexão com status válido daquele
  perfil.
- O app não pede nem exibe senha, token ou código de autorização. O login e o status não modificam
  nem usam como alternativa o login padrão ou o de outro perfil.

## Edge cases

| #   | Quando                                                                            | O sistema deve                                                                                                                                                 |
| --- | --------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Dois perfis são cadastrados e autenticados, e o app é reaberto                    | Manter os cadastros separados, associar cada status ao perfil correto e deixar o login padrão como estava.                                                     |
| 2   | Login é cancelado, negado ou não confirmado pelo status                           | Não declarar o perfil conectado; exibir o estado ou a falha correspondente e permitir nova tentativa.                                                          |
| 3   | CLI ausente, incompatível ou com falha em obter status                            | Mostrar erro acionável no perfil, sem tratar erro como desconexão confirmada nem recorrer ao login padrão.                                                     |
| 4   | O usuário inicia login novamente enquanto há um login em andamento naquele perfil | Manter um só fluxo por perfil e indicar que já está em andamento.                                                                                              |
| 5   | Saída da CLI contém dados sensíveis ou campos desconhecidos                       | Mostrar somente dados permitidos; não expor dados sensíveis em erros ou diagnósticos, nem incluir o estado privado do perfil em editor, biblioteca ou backups. |

## Out of scope

Importar ou copiar login existente (inclusive o padrão), trocar a conta ativa do Claude Code fora do
app e monitorar cotas ([SPEC-0007](0007-monitorar-cotas-por-conta-do-claude-code.md)).

## Open questions

Nenhuma para o contrato aprovado de cadastro.

## Definition of Done

Usar os runners existentes abaixo. Os alvos dedicados são **futuros** e devem ser adicionados na
implementação; cada comando deve sair com código zero, executando todos os casos indicados sem
pulá-los. Usar CLI simulada e credenciais fictícias na automação.

```bash
cargo test --workspace --test claude_account_profiles                       # casos 1–5: isolamento, status, concorrência e fronteira dos dados
bun run test:ts src/features/claude-accounts/ClaudeAccountProfiles.test.tsx # casos 1–5: fluxo e apresentação dos estados
```

Com `bun run dev` no Linux, registrar o status padrão fora do app sem copiar credenciais. Cadastrar
duas contas de teste distintas pelo fluxo oficial, reabrir o app e confirmar que os dois perfis
seguem distintos e que o status e a identidade padrão continuam os mesmos (caso 1).

## Human review

Conferir que a UI distingue autenticação pendente, falha e sucesso e sinaliza que a autenticação
ocorre na experiência oficial.

## Verification

Pendente de implementação e de execução do DoD funcional.
