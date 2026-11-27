---
status: accepted
date: 2026-11-27
builds-on: [ADR-0004, ADR-0005, ADR-0006, ADR-0007, SPEC-0004]
implemented-by: []
---

# Editar configurações globais de agentes via SSH

## Goal

Abrir e editar arquivos nativos existentes de Claude Code e Codex para a conta Linux autenticada no
ambiente da [SPEC-0004](0004-cadastrar-e-inspecionar-ambientes-ssh.md), sem mudar o fluxo local da
[SPEC-0001](0001-global-agent-config-inspection-and-safe-editing.md).

## Flow

1. O usuário seleciona um ambiente SSH, um agente detectado e um arquivo global suportado.
2. O editor abre o texto nativo e mascara por padrão os valores sensíveis que forem identificados. O
   usuário altera o texto e revisa o diff com host, conta, porta e caminho remoto absolutos.
3. Após confirmação explícita, o app valida novamente e tenta salvar a versão revisada. Mostra
   sucesso confirmado, conflito, falha ou resultado não confirmado. Nenhum desses fluxos salva no
   desktop como alternativa ao destino remoto.

## Contract

Lista inicial, derivada dos caminhos padrão dos adapters locais, restrita a arquivos existentes:

| Agente      | Arquivo relativo ao home remoto | Formato |
| ----------- | ------------------------------- | ------- |
| Claude Code | `.claude/settings.json`         | JSON    |
| Claude Code | `.claude.json`                  | JSON    |
| Claude Code | `.claude/.claude.json`          | JSON    |
| Codex       | `.codex/config.toml`            | TOML    |

- O backend deriva o caminho de um identificador de arquivo permitido; não aceita um caminho
  informado pelo editor. Caminhos personalizados por variáveis de ambiente, arquivos de credenciais
  e links simbólicos abaixo do home resolvido não são suportados.
- A edição é textual: gravar o texto validado, sem reformatar ou reserializar automaticamente. A
  validação de sintaxe não certifica a configuração do agente. Campos por projeto existentes dentro
  de um arquivo global compartilhado do Claude Code, como `projects`, não podem ser alterados por
  este fluxo; a proteção é aplicada no backend mesmo com editor textual.
- O diff aprovado vale somente para a versão exibida e para o destino autenticado da leitura.
  Mudança de conteúdo, cadastro, identidade ou alvo invalida a confirmação. Os bytes originais são
  preservados em backup privado conforme a ADR-0005; não há restauração remota na UI.
- Conflito detectado antes da troca bloqueia a escrita; não há modo de forçar sobrescrita. Não há
  vigilância contínua, nem garantia de exclusão de editores externos simultâneos.

## Edge cases

| #   | Quando                                                                          | O sistema deve                                                                                                                              |
| --- | ------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | O arquivo existe e é legível                                                    | Abrir em JSON/TOML, permitir revisão do diff e salvar apenas o texto confirmado no destino exibido.                                         |
| 2   | O arquivo está ausente, é ilegível ou não é texto UTF-8                         | Informar a condição sem criar arquivo, diretório ou permitir sobrescrita.                                                                   |
| 3   | O rascunho tem sintaxe inválida ou altera um bloco protegido de projeto         | Mostrar erro e rejeitar a escrita também no backend.                                                                                        |
| 4   | Há valores sensíveis identificados                                              | Mascarar no editor e diff por padrão, sem substituir os bytes reais por máscaras nem registrá-los em logs.                                  |
| 5   | O original mudou, foi removido ou substituído desde a abertura                  | Rejeitar a gravação quando a revalidação detectar a divergência, manter rascunho e oferecer releitura, sem sobrescrita forçada.             |
| 6   | O destino mudou, o arquivo não está na lista permitida ou há link simbólico     | Rejeitar a operação incompatível e exigir nova abertura se o destino mudou.                                                                 |
| 7   | Backup falha, o host não suporta a troca segura, ou ocorre falha antes da troca | Informar falha, manter o rascunho e não deixar o alvo parcialmente gravado.                                                                 |
| 8   | A conexão cai sem confirmação da gravação                                       | Mostrar resultado não confirmado, preservar rascunho e oferecer verificação por nova leitura no mesmo destino, sem repetir automaticamente. |

## Out of scope

Instalação, provisionamento, Antigravity, edição de projetos, arquivos separados de skills e regras,
formulários por recurso e sincronização entre máquinas ou com a biblioteca compartilhada.

## Open questions

- [x] Validar as primitivas mínimas do host para a troca atômica com revalidação e preservação de
      permissões, e registrar a corrida residual em teste antes de aceitar o fluxo de escrita.

A prova em `crates/core/tests/ssh_atomic_probe.rs` validou as operações Linux de Python 3 relativas
a um descritor de diretório, SHA-256, temporário exclusivo, permissões e atributos estendidos,
`fsync` e `rename` na mesma pasta. Uma alteração anterior à revalidação foi rejeitada, enquanto uma
alteração entre a revalidação e o `rename` pôde ser sobrescrita. O teste isolado cobriu modo, UID,
GID e um atributo `user.*`.

## Definition of Done

Os runners abaixo já existem; adicionar as suítes nomeadas durante a implementação. Cada comando
precisa sair com código zero e executar os casos indicados, sem pulá-los. Os testes de transporte
usam um host Linux isolado com OpenSSH e arquivos sintéticos, nunca os do usuário.

```bash
cargo test --workspace --test ssh_config_edit                    # casos 1–8; inclui corrida, falhas de troca e queda de conexão
bun run test:ts src/features/configs/RemoteConfigEditor.test.tsx # casos 1–8; diff, erros e estados de recuperação
```

## Human review

Conferir a identificação do destino na prévia e se a interface distingue conflito de resultado não
confirmado sem induzir nova escrita.

## Verification

Pendente de implementação e execução do DoD funcional.
