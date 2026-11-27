---
status: accepted
date: 2026-11-27
builds-on: [ADR-0006]
implemented-by: []
---

# Cadastrar e inspecionar ambientes SSH

## Goal

Cadastrar destinos SSH Linux e verificar se Claude Code e Codex estão disponíveis ali, sem misturar
esse inventário com o desktop. Esta entrega é utilizável sem o editor remoto.

## Flow

1. No seletor de ambientes, o usuário escolhe `Local` ou adiciona um ambiente SSH por alias do
   OpenSSH ou por host, usuário e porta.
2. O usuário pode testar a conexão sem gravar o cadastro. O resultado mostra o destino efetivo
   autenticado ou uma falha com orientação. Salvar mantém o cadastro no desktop.
3. Ao selecionar um cadastro, o app carrega o inventário remoto. O usuário pode atualizá-lo, editar
   o cadastro ou removê-lo sem modificar o servidor.

## Contract

- O cadastro guarda nome de exibição e um modo: alias ou host, usuário e porta (padrão 22). Não
  guarda senhas, passphrases ou chaves privadas. Autenticação interativa e aprovação de novas chaves
  de servidor acontecem fora do aplicativo, no OpenSSH.
- Detectado significa que `claude` ou `codex` está disponível como executável no `PATH` da sessão
  SSH não interativa; não prova login, funcionamento ou ausência de instalação fora do `PATH`. A
  descoberta não executa os agentes.
- Resultados pertencem ao ambiente consultado. Alterar a conexão ou trocar de ambiente invalida
  resultados pendentes desse contexto. O inventário local existente não muda.

## Edge cases

| #   | Quando                                                                                  | O sistema deve                                                                                      |
| --- | --------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1   | Alias ou cadastro manual válidos                                                        | Testar os dois formatos, mostrar host, conta e porta efetivos e manter o cadastro ao reabrir o app. |
| 2   | Campos faltantes, porta fora de 1–65535 ou entrada interpretável como opção SSH         | Recusar antes de iniciar a conexão.                                                                 |
| 3   | Cliente indisponível, autenticação recusada, chave desconhecida ou alterada, ou timeout | Mostrar falha sem aceitar chaves automaticamente nem marcar os agentes como ausentes.               |
| 4   | Um ou ambos os executáveis não estão no `PATH`, ou a consulta falha                     | Mostrar o estado de cada agente e o limite da descoberta; falha de consulta não é ausência.         |
| 5   | O ambiente é trocado, alterado ou removido durante o carregamento                       | Descartar resposta antiga; se o ambiente ativo for removido, voltar explicitamente para `Local`.    |

## Out of scope

Instalar agentes, provisionar hosts, oferecer terminal, gerir credenciais ou editar arquivos
remotos. Edição: [SPEC-0005](0005-editar-configuracoes-globais-de-agentes-via-ssh.md).

## Open questions

Nenhuma para o contrato desta entrega.

## Definition of Done

Os runners abaixo já existem; adicionar as suítes nomeadas durante a implementação. Cada comando
precisa sair com código zero e executar os casos indicados, sem pulá-los. Usar OpenSSH com um
servidor Linux isolado e chaves e arquivos descartáveis, nunca hosts cadastrados pelo usuário.

```bash
cargo test --workspace --test ssh_inventory                            # casos 1–4, incluindo os dois modos com OpenSSH real
bun run test:ts src/features/environments/EnvironmentSwitcher.test.tsx # casos 1–5
```

## Human review

Conferir se ambiente ativo e erros de autenticação ou identidade ficam inequívocos na UI.

## Verification

Pendente de implementação e execução do DoD funcional.
