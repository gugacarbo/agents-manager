---
status: draft
date: 2026-11-27
builds-on: [ADR-0010]
implemented-by: []
---

# Aplicar preferências de interface e MCPs entre perfis do Claude Desktop

## Goal

Reutilizar preferências de interface e configurações de MCPs de um perfil de referência do Claude
Desktop em outros perfis selecionados no Linux.

## Flow

1. O usuário escolhe um perfil de referência entre os perfis cadastrados, incluindo a sessão padrão.
   Pode mudar essa escolha sem aplicar configurações.
2. Em **Aplicar configurações**, o usuário seleciona os destinos e revisa os valores atuais e
   propostos por perfil. Pode escolher preferências de UI e servidores MCP individualmente.
3. O usuário confirma a seleção com o Desktop fechado. O Manager apresenta o resultado por destino.
4. O usuário pode desfazer a aplicação em um destino. Uma nova aplicação exige nova prévia.

## Contract

- A interface apresenta apenas preferências com suporte verificado. Tema, idioma e zoom são
  candidatos para verificação no Desktop, quando a versão oferecer controle local desses valores.
- Cada destino pode manter exceções persistentes por preferência ou servidor. A operação muda
  somente itens selecionados e elegíveis. Ausência de um item na referência não apaga o item local.
- A prévia apresenta diferenças sem revelar valores secretos. Um MCP com credencial literal fica
  indisponível para cópia. Referências a variáveis permitidas permanecem referências, e cada destino
  resolve seus valores no próprio ambiente.
- Credenciais, cookies, estado de login, sessões, histórico e outros campos fora da seleção
  permanecem no perfil de origem. Backups e desfazimento seguem a projeção da ADR-0010.
- Uma mudança nos arquivos envolvidos após a prévia invalida a seleção afetada. Desfazer restaura
  somente valores gravados pela aplicação, se ainda corresponderem ao resultado da aplicação.

## Edge cases

| #   | Quando                                                                                                      | O sistema deve                                                                                                                  |
| --- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| 1   | A referência tiver preferências de UI verificadas ou MCPs elegíveis                                         | Mostrar as diferenças e aplicar apenas os itens selecionados aos destinos escolhidos.                                           |
| 2   | Um destino tiver preferências próprias, exceções ou itens ausentes na referência                            | Preservar esses valores e mostrar as exceções na prévia.                                                                        |
| 3   | Um arquivo misturar campos de UI, campos desconhecidos e dados privados, ou um MCP contiver segredo literal | Preservar campos inelegíveis, bloquear aquele MCP e manter os segredos fora da prévia, logs e desfazimento.                     |
| 4   | O Desktop estiver aberto, o formato for incompatível ou a referência ou destino mudar após a prévia         | Impedir a aplicação afetada e informar o que precisa ser resolvido ou revisado.                                                 |
| 5   | Uma gravação falhar em um destino                                                                           | Reverter as alterações parciais daquele destino ou indicar recuperação pendente, preservando os resultados dos outros destinos. |
| 6   | O usuário desfizer a aplicação após o destino ter sido editado                                              | Restaurar campos ainda compatíveis com a aplicação e informar conflitos sem sobrescrever edições posteriores.                   |
| 7   | O usuário trocar de conta ou apenas mudar a referência                                                      | Preservar as configurações de todos os perfis até uma confirmação explícita de aplicação.                                       |

## Out of scope

Sincronização automática em segundo plano, sincronização bidirecional, preferências da conta
armazenadas exclusivamente na nuvem e preferências do próprio Agents Manager.

## Open questions

- [ ] Identificar a lista inicial de preferências de UI locais e os formatos de MCP que podem ser
      lidos, aplicados e revertidos com segurança na versão do Desktop para Linux. Registrar
      evidência por campo, incluindo a verificação de tema, idioma e zoom.

## Definition of Done

Estender a suíte Rust de perfis com arquivos fictícios mistos, conflitos e falhas de gravação.
Estender a suíte da tela de contas com seleção, prévia, aplicação e desfazimento.

```bash
cargo test --workspace --test claude_account_quotas                       # exit 0, casos 1–7
bun run test:ts src/features/claude-accounts/ClaudeAccountQuotas.test.tsx # exit 0, casos 1–7
python3 scripts/docs-check                                                # exit 0
```

Com `bun run dev`, selecionar dois perfis com preferências diferentes. Aplicar pelo menos uma
preferência real de UI e um MCP de teste elegível, abrir o destino e confirmar os dois efeitos.
Desfazer a aplicação, reabrir e confirmar a restauração e a continuidade dos logins.

## Human review

Conferir que a prévia identifica destinos, exceções, alterações e itens indisponíveis antes da
confirmação.
