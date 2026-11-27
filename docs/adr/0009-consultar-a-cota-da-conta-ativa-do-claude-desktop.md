---
status: accepted
date: 2026-11-27
builds-on: [ADR-0008]
superseded-by: null
deciders: []
---

# Consultar cotas de contas do Claude Desktop por perfil isolado

## Context and Problem

A [SPEC-0007](../specs/0007-monitorar-cotas-por-conta-do-claude-code.md) precisa consultar a cota de
várias contas do Claude Desktop no Linux com o cliente fechado. A integração inicial comprovou a
leitura da conta padrão pela fonte OAuth não pública do Desktop. A ampliação precisa dar a cada
conta sua própria sessão e manter a atribuição da leitura verificável.

## Decision Drivers

- Permitir consultar cada conta cadastrada mesmo quando ela não está ativa na sessão padrão.
- Manter o Desktop responsável pelo login, pelo armazenamento e pela renovação das credenciais.
- Preservar o vínculo e a leitura já salvos da sessão padrão e o isolamento da CLI da ADR-0008.

## Considered Options

### Option 1 — Usar perfis locais do próprio Claude Desktop

**Pros:** Cada conta tem uma sessão administrada pelo Desktop e um diretório próprio de dados.
**Cons:** Depende do suporte do Desktop instalado a diretórios de usuário isolados no Linux e de
interfaces OAuth não públicas.

### Option 2 — Salvar e restaurar cópias de sessões, como o Switcher

**Pros:** Permite reusar sessões e alternar a conta na instalação padrão. **Cons:** Exige custodiar
cópias de sessões e alterar os arquivos de login para trocar de conta.

### Option 3 — Delegar a consulta ao Switcher

**Pros:** Usa o cadastro de contas já disponível no Switcher. **Cons:** Cria uma dependência
operacional em outro aplicativo e em seus formatos privados.

## Decision

Usar um perfil local do próprio Claude Desktop por conta adicionada. O Agents Manager prepara
diretórios XDG privados e inicia o Desktop com esse ambiente quando o usuário solicita login. O
usuário se autentica na interface oficial. O Desktop é responsável pela sessão e pelos arquivos de
autenticação de cada perfil. O vínculo padrão existente continua associado à instalação padrão e
mantém os resultados já salvos.

Com o Desktop fechado, o backend abre a fonte de um único perfil por operação. Usa apenas o mínimo
do cache OAuth desse perfil para confirmar a identidade e consultar a cota, com o token em memória
durante a operação. O vínculo e os metadados de cotas ficam no armazenamento do Agents Manager.
Credenciais, cookies e caches do Desktop permanecem sob responsabilidade do Desktop, fora do IPC,
dos logs, do editor, da biblioteca, da sincronização e dos backups gerenciados pelo app.

Quando a sessão expira, o usuário pode abrir o login do próprio perfil. A coleta nunca renova
tokens, abre o Desktop ou recorre a outra conta. O fluxo gerenciado não troca o login da instalação
padrão e não acessa perfis da CLI.

## Consequences

Mudanças no formato do Desktop ou na interface OAuth podem inutilizar uma fonte. O backend só
atribui uma leitura se confirmar a identidade vinculada no próprio perfil. Uma falha é registrada
por conta e preserva o horário do último sucesso.

## Confirmation

Executar a prova com dois perfis autenticados no Linux conforme a SPEC-0007. Inspecionar o backend e
testar com dados fictícios que o IPC e a persistência gerenciada recebem apenas os metadados
permitidos. Conferir que uma identidade divergente ou uma fonte indisponível não causam leitura de
outra conta e que os diretórios privados ficam fora dos fluxos de configuração do aplicativo.
