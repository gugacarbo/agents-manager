---
status: draft
date: 2026-11-27
builds-on: [ADR-0008]
implemented-by: []
---

# Monitorar cotas por conta do Claude Code

## Goal

Consultar, no desktop Linux, as cotas de cada conta cadastrada mesmo quando não há sessões do Claude
Code abertas. O cadastro de contas é tratado separadamente em
[SPEC-0006](0006-cadastrar-contas-do-claude-code-em-perfis-isolados.md).

## Flow

1. Ao abrir a tela de monitoramento, o app tenta consultar as contas elegíveis separadamente.
2. O botão de atualização dispara nova consulta para as contas elegíveis, respeitando o intervalo
   por conta. Não há atualização periódica, em segundo plano ou reagendada automaticamente.
3. Cada cartão identifica a conta e mostra os dados retornados ou o estado da consulta.

## Contract

- Para cada conta, mostrar uso e horário de renovação das janelas de 5h e 7d quando disponíveis.
  Campos ausentes são indisponíveis, não zero; a ausência de um campo não oculta os demais.
- A consulta é vinculada à conta autenticada, não ao processo, à sessão ou ao projeto ativo. A falha
  ou limitação de uma conta não impede atualizar outra elegível.
- Há no mínimo 60 segundos entre o início de duas tentativas para a mesma conta, inclusive após
  falha. Não iniciar uma segunda consulta enquanto outra da mesma conta estiver em andamento, mesmo
  após 60 segundos. O limite vale igualmente para abertura e botão, sem enfileirar consultas
  bloqueadas.
- Mostrar o horário da última tentativa, se houve, e o horário da última coleta bem-sucedida, se
  houver dados. Uma falha mostra o erro da conta. Se a UI mantiver dados da coleta anterior após
  falha ou sem nova coleta, apresenta-os como históricos, com o horário original, não como atuais.

## Edge cases

| #   | Quando                                                                                                                     | O sistema deve                                                                                                                                                                                                    |
| --- | -------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Duas contas conectadas, sem sessões abertas, e o usuário abre a tela ou aciona o botão com ambas elegíveis                 | Consultar e atribuir uso e renovação somente à conta correta.                                                                                                                                                     |
| 2   | A fonte retorna apenas parte dos campos ou nenhuma das janelas                                                             | Mostrar o que foi retornado e indicar o restante como indisponível, sem estimar dados.                                                                                                                            |
| 3   | A consulta falha ou a conta deixa de estar autenticada                                                                     | Mostrar o erro daquela conta, sem promover cache a leitura atual ou impedir a consulta de outra conta. Se nunca houve sucesso, não mostrar dados de cota.                                                         |
| 4   | Abre a tela ou clica em atualizar antes de 60 segundos, inclusive após falha, ou durante outra consulta para a mesma conta | Não iniciar nem agendar tentativa adicional; mostrar que a consulta está em andamento ou que é necessário aguardar. A partir de 60 segundos e sem consulta em andamento, aceitar o próximo acionamento explícito. |
| 5   | A tela permanece aberta ou é fechada sem novo acionamento explícito                                                        | Não iniciar consulta em segundo plano nem ao expirar o intervalo.                                                                                                                                                 |

## Out of scope

Monitoramento de sessão ou projeto, alertas, coleta em segundo plano e estimativa de consumo.

## Open questions

- [ ] **Bloqueante:** qual fonte permite obter e atribuir cotas às contas cadastradas, sem sessão
      aberta no Claude Code e sem quebrar a fronteira de autenticação da ADR-0008? Antes da
      implementação, executar a prova manual no Linux descrita abaixo. Registrar mecanismo e versão
      usados, resultado por conta, janelas e campos disponíveis, evidência da atribuição e limites
      observados, sem segredos. Sem evidência reproduzível, a Spec continua em rascunho e o
      monitoramento não é implementado.

## Definition of Done

**Prova manual da fonte (pré-requisito, caso 1):** no Linux, usar duas contas de teste distintas,
com login em perfis separados. Encerrar todas as sessões do Claude Code, inclusive as de segundo
plano. Consultar pelo mecanismo candidato com cada perfil, sem iniciar uma sessão. Confirmar a
identidade independentemente da consulta de cotas e cruzar os dados retornados com uma leitura
confiável por conta; repetir alternando A → B → A para verificar a atribuição. A prova passa se
obtiver uma leitura de cota atribuível a cada conta, com uso e horário de renovação nas janelas
expostas, sem abrir sessão ou expor credenciais. Registrar campos ausentes, sem inferi-los.

Após validação, usar os runners existentes abaixo. Os alvos dedicados são **futuros**; cada comando
precisa sair com código zero e executar os casos indicados sem pulá-los. Usar fonte simulada e
relógio controlado para as verificações automatizadas.

```bash
cargo test --workspace --test claude_account_quotas                       # casos 1–5: escopo, dados parciais, falha, intervalo e concorrência
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # casos 1–5: gatilhos, estados e horários por conta
```

Com `bun run dev`, repetir no app Linux o cenário de duas contas sem sessões e conferir uso e
renovação contra a prova da fonte; verificar tentativa sem sucesso com erro e horário, sem
apresentar o último sucesso como dado atual (casos 1–3).

## Human review

Conferir que, em cada cartão, identidade, horário da coleta, indisponibilidade e erro podem ser
entendidos sem confundir valores históricos com atuais.

## Verification

Fonte não validada. Prova manual, implementação e execução do DoD funcional pendentes.
