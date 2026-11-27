---
status: draft
date: 2026-11-27
builds-on: [ADR-0011, SPEC-0008]
implemented-by: []
---

# Importar conversas locais e SSH ao trocar contas do Claude Desktop

## Goal

Retomar na conta de destino conversas selecionadas do Claude Code local ou via SSH.

## Flow

1. Ao trocar de conta, o usuário pode habilitar a importação para o par de perfis escolhido.
2. Com o Desktop fechado, o Manager mostra as sessões disponíveis, sua origem e o destino.
3. O usuário seleciona as conversas e confirma a importação. A seleção de importar é independente da
   confirmação de trocar.
4. O Manager informa os resultados e abre o perfil de destino. Uma falha de importação permite
   repetir a importação ou seguir com a troca.

## Contract

- Importar somente sessões selecionadas. A listagem identifica origem local ou ambiente SSH
  cadastrado. A conta de destino precisa conseguir acessar a sessão para retomá-la.
- Uma segunda importação da mesma sessão não cria outra entrada. Preservar exclusões do destino e
  atualizações posteriores. Sessões em execução ficam indisponíveis para importação até poderem ser
  retomadas sem concorrência.
- Uma falha por sessão ou host não apaga referências existentes. A interface distingue sessões
  importadas, ignoradas e indisponíveis.

## Edge cases

| #   | Quando                                                                           | O sistema deve                                                                                                                  |
| --- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Uma sessão local acessível for selecionada                                       | Torná-la disponível no perfil de destino com o histórico correspondente, inclusive quando os diretórios locais forem separados. |
| 2   | Uma sessão SSH for selecionada                                                   | Preservar o host e o diretório de origem e permitir retomada com o acesso do destino.                                           |
| 3   | Um host não responder ou o destino não tiver acesso ao diretório                 | Informar a origem indisponível, preservar as referências anteriores e permitir a troca.                                         |
| 4   | A sessão já existir, tiver sido excluída ou tiver edição mais recente no destino | Evitar duplicação, restauração involuntária e sobrescrita da versão mais recente.                                               |
| 5   | Uma sessão estiver ativa ou uma gravação falhar                                  | Preservar a sessão e os dados anteriores, indicando o resultado individual.                                                     |
| 6   | A importação estiver desabilitada ou o usuário cancelar a seleção                | Não importar conversas e deixar disponível a troca simples.                                                                     |

## Out of scope

Histórico de conversas armazenadas na nuvem do Claude, replicação de transcrições SSH e importação
automática de todo o histórico entre contas.

## Open questions

- [ ] Verificar, com o Desktop instalado no Linux, os formatos de índice e os requisitos de retomada
      local e SSH que serão suportados pela primeira entrega.

## Definition of Done

Adicionar à suíte Rust testes de importação com diretórios temporários e transporte SSH simulado.
Adicionar à suíte da tela de contas testes de seleção e resultados.

```bash
cargo test --workspace --test claude_account_quotas                       # exit 0, casos 1–6
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # exit 0, casos 3–6
python3 scripts/docs-check                                                # exit 0
```

Com `bun run dev` e duas contas vinculadas, iniciar uma conversa local e outra via SSH na conta A,
importá-las ao trocar para B, retomar cada conversa e enviar uma nova mensagem. Conferir histórico
correto e repetir a importação para verificar que a lista não contém duplicatas.

## Human review

Conferir se a revisão identifica claramente a origem e a conta de destino antes de importar.
