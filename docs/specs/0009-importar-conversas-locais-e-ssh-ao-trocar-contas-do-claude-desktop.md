---
status: accepted
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

Nenhuma após a prova de compatibilidade abaixo.

## Definition of Done

Adicionar à suíte Rust testes de importação com diretórios temporários e transporte SSH simulado.
Adicionar à suíte da tela de contas testes de seleção e resultados.

```bash
cargo test --workspace --test claude_account_quotas                       # exit 0, regressão de perfis e cotas
cargo test --workspace --test claude_conversation_import                  # exit 0, casos 1–6
cargo test --workspace --test ssh_sessions                                # exit 0, casos 2–3 e 5
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # exit 0, casos 3–6
python3 scripts/docs-check                                                # exit 0
```

Com `bun run dev` e duas contas vinculadas, iniciar uma conversa local e outra via SSH na conta A,
importá-las ao trocar para B, retomar cada conversa e enviar uma nova mensagem. Conferir histórico
correto e repetir a importação para verificar que a lista não contém duplicatas.

## Human review

Conferir se a revisão identifica claramente a origem e a conta de destino antes de importar.

## Verification

Em 2026-11-27, no Linux com o Claude Desktop 6.28899.6, foram criadas duas conversas descartáveis na
conta padrão e inseridas suas entradas de índice no perfil isolado de outra conta, com o Desktop
fechado. O índice usa `claude-code-sessions/<conta>/<org>/local_<id>.json`. A primeira passagem
gravou duas entradas e a segunda gravou zero.

- **Local.** A conta de destino exibiu o histórico com o mesmo `cliSessionId`. Na primeira tentativa
  de enviar, o Git bloqueou o checkout da branch já em uso no worktree de origem. Preservar
  `worktreePath`, `worktreeName` e `folderPlace`, além da branch e do diretório, permitiu retomar a
  conversa usando o worktree original. A nova mensagem e a resposta `SPEC9-OK` foram verificadas na
  transcrição original.
- **SSH.** A entrada preservou `sshConfig.sshHost`, `cwd` e `sshRemoteTranscriptPath`. O Desktop
  exibiu o histórico e gravou a nova mensagem e a resposta `SPEC9-OK` na transcrição original no
  host. O acesso ao diretório de trabalho e a leitura da transcrição foram verificados antes da
  importação. Em uma amostra anterior de 20 referências do mesmo host, todos os JSONL estavam
  legíveis e somente três diretórios de trabalho estavam acessíveis.

Nas duas retomadas, o Desktop avisou que o raciocínio anterior pertencia a outra organização. O
histórico visível permaneceu disponível e ambas as conversas responderam.

**Diretórios locais separados.** Com o Desktop fechado, a transcrição da sessão local foi copiada
para um diretório temporário de destino, preservando o caminho relativo em `projects` e o
`cliSessionId`. O perfil isolado foi reaberto com `CLAUDE_CONFIG_DIR` apontando para esse diretório.
O Desktop exibiu o histórico e respondeu `SPEC9-SEPARADO-OK`. A nova mensagem e a resposta
apareceram na cópia do mesmo JSONL no destino. O JSONL de origem não recebeu o novo turno. Na versão
examinada, o Desktop resolve primeiro `CLAUDE_CONFIG_DIR` salvo nas variáveis de ambiente do perfil,
em `ccd-environment-config.json`, depois a variável do processo e por fim `~/.claude`. A prova
configurou a variável do processo.

As provas usaram entradas adicionadas diretamente ao índice. Ainda faltam o fluxo de importação no
Manager e a cobertura do DoD.
