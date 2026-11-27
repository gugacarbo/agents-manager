---
status: accepted
date: 2026-11-27
builds-on: [ADR-0008, ADR-0009]
implemented-by: []
---

# Monitorar cotas de várias contas do Claude Desktop

## Goal

No Linux, cadastrar várias contas do Claude Desktop e consultar cada uma com o cliente fechado,
incluindo contas de perfis isolados que não estão ativos na instalação padrão.

## Flow

1. Na tela de contas do Desktop, o usuário escolhe **Adicionar conta**. Abre o login oficial em um
   perfil isolado. Depois de concluir a autenticação e fechar o Desktop, confirma o vínculo. Essa
   confirmação verifica a identidade independentemente da resposta de cota.
2. O vínculo existente e seu histórico aparecem como a conta da sessão padrão. Em instalações novas,
   o usuário pode vincular a sessão padrão explicitamente.
3. Ao abrir a tela, ao escolher **Atualizar todas** ou ao escolher **Atualizar** em uma conta,
   consultar as contas elegíveis com o Desktop fechado. Cada conta mostra o resultado da própria
   tentativa.
4. Quando uma sessão isolada expira, o usuário pode abrir o login desse perfil novamente e confirmar
   a mesma identidade. Nenhum login é iniciado pelo ato de atualizar a cota.

## Contract

- Cada conta tem uma identidade estável e uma única cota. Quando a mesma identidade for confirmada
  na sessão padrão e em um perfil isolado, mostrar uma única conta com as duas opções de abertura. O
  vínculo padrão conserva seu histórico e é a fonte de atualização do par. A confirmação do perfil
  isolado verifica a identidade na sua própria sessão. Uma segunda sessão isolada para a mesma
  identidade continua sendo rejeitada.
- Verificar a identidade a cada coleta no perfil do próprio vínculo. Erros e indisponibilidade de
  uma conta não impedem a atualização de outras e não provocam consulta a fontes alternativas.
- Mostrar o percentual usado e o horário de renovação das janelas de 5h e 7d. Tratar cada campo
  ausente como indisponível e mostrar campos presentes, inclusive zero.
- Aplicar por conta o intervalo mínimo de 60 segundos entre inícios de tentativas, inclusive após
  erro e reabertura. Bloquear tentativas simultâneas para a mesma conta. A atualização em lote
  respeita as mesmas regras e descarta tentativas bloqueadas sem enfileirar novas.
- Exibir os horários da última tentativa e da última coleta bem-sucedida quando existirem. Dados
  recuperados do disco ou mantidos após falha ou bloqueio aparecem como históricos.
- A autenticação e a coleta em perfis isolados preservam a sessão padrão. O app persiste identidade,
  vínculo e metadados de cota, seguindo a fronteira de credenciais da ADR-0009.

## Edge cases

| #   | Quando                                                                                                      | O sistema deve                                                                                                                          |
| --- | ----------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | O usuário adiciona dois perfis, conclui os logins e fecha o Desktop                                         | Manter dois vínculos independentes, verificar cada identidade e atualizar ambas as cotas sem abrir o Desktop ou alterar o login padrão. |
| 2   | A fonte devolve horários ou percentuais parciais                                                            | Mostrar os campos presentes e indicar os demais como indisponíveis.                                                                     |
| 3   | Uma conta falha ou expira durante a atualização em lote                                                     | Preservar o histórico da conta, mostrar o erro e continuar as outras contas elegíveis.                                                  |
| 4   | Uma atualização manual e outra em lote pedem a mesma conta dentro do intervalo ou com consulta em andamento | Bloquear apenas aquela conta sem reagendar a solicitação.                                                                               |
| 5   | A tela permanece aberta, é fechada ou recupera foco sem novo acionamento explícito                          | Manter os dados exibidos sem iniciar coleta em segundo plano.                                                                           |
| 6   | A identidade diverge, a fonte está indisponível ou já existe um perfil incompatível para a conta            | Mostrar erro sem atribuir nova leitura, criar um vínculo duplicado ou consultar outra conta.                                            |
| 7   | O app abre com o arquivo da versão de conta única                                                           | Exibir a conta da sessão padrão e os dados anteriormente salvos como históricos até obter nova leitura.                                 |
| 8   | O usuário fecha a tela ou reinicia o app durante o cadastro                                                 | Manter o perfil pendente e permitir retomar o fluxo oficial sem perder vínculos anteriores.                                             |

## Out of scope

Coleta da CLI, troca automática de conta no Desktop, importação de sessões do Switcher, alertas e
coleta periódica.

## Open questions

Nenhuma de produto. A disponibilidade da fonte para duas contas reais é verificada no DoD.

## Definition of Done

Os testes automatizados usam fonte simulada e relógio controlado.

```bash
cargo test --workspace --test claude_account_quotas                       # casos 1–4 e 6–8
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # casos 1–8
python3 scripts/docs-check
```

Com `bun run dev` no Linux, cadastrar duas contas de teste no Desktop, fechar o cliente e coletar a
cota de cada conta no app. Comparar percentuais e horários de renovação com uma referência de cada
conta e verificar que configuração e vínculo padrão seguem intactos, inclusive após reabrir.

## Human review

Conferir que a tela identifica cada conta, seu estado de autenticação, o resultado da atualização em
lote e a idade dos dados.

## Verification

- No Linux, o Claude Desktop 6.28899.6 abriu dois perfis XDG privados pelo botão **Add account**.
  Cada perfil recebeu seu `config.json`, e o arquivo da sessão padrão manteve o mesmo conteúdo. Os
  dois cadastros reapareceram depois de navegar para fora da tela. A confirmação de um perfil sem
  sessão autenticada mostrou erro e manteve o cadastro pendente.
- No app nativo, **Refresh all** coletou a cota da conta padrão e deixou de fora os dois perfis
  pendentes. Ainda falta repetir a coleta com duas contas reais autenticadas em perfis separados.
- Passaram os 8 testes do alvo Rust `claude_account_quotas` e os 8 de
  `ClaudeAccountQuotas.test.tsx`. Passaram também `validate:fast`, a formatação, os builds web e
  Storybook e as suítes Storybook e Playwright. Os demais alvos Rust passaram com os três testes que
  exigem `sshd` filtrados.

### Entrega inicial com a conta padrão

- Em 2026-11-27, no Linux com o Claude Desktop 6.28899.6 fechado, o perfil confirmou a identidade da
  conta ativa independentemente da cota. A leitura direta das 10h29 (horário de Brasília) confirmou
  os percentuais do registro do Switcher das 10h26 para a mesma conta. A janela de 5h retornou 0%
  sem horário de renovação nas duas leituras. A de 7d retornou 100% e renovação em 30/11 por volta
  das 19h, com cerca de um segundo de diferença entre as fontes.
- A consulta retornou HTTP 429 nas duas tentativas com o Switcher em execução. A leitura
  bem-sucedida foi obtida com ambos os clientes fechados. O teste não alterou o login nem a
  configuração do Desktop.

- O alvo Rust `claude_account_quotas` passou com 4 testes, cobrindo vínculo sem coleta,
  persistência, divergência de identidade, falha, intervalo e concorrência entre instâncias. O alvo
  `ClaudeAccountQuotas.test.tsx` passou com 9 testes, cobrindo os seis casos da Spec na interface.
- Em 2026-11-27, no app nativo iniciado com `bun run dev` e dados XDG de teste, o vínculo foi
  concluído sem tentativa de cota. Às 10h42, com os dois clientes fechados, a coleta retornou 0% na
  janela de 5h, com renovação indisponível, e 100% na janela de 7d, com renovação em 30/11 às 19h
  (horário de Brasília). Uma segunda solicitação antes de 60 segundos mostrou a espera e preservou a
  leitura como histórica. Ao reabrir o app com a fonte indisponível, a interface mostrou o erro e
  manteve a leitura histórica com o horário original.
- `python3 scripts/docs-check`, `bun run validate:fast`, `bun run format:check`,
  `bun run build:web`, `bun run test:storybook`, `bun run build-storybook` e `bun run test:e2e`
  passaram. A suíte global de Rust depende de `sshd`, ausente neste ambiente. Com os três testes que
  iniciam um servidor SSH local filtrados, todos os demais alvos do workspace passaram.
