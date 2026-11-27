---
status: proposed
date: 2026-11-27
builds-on: [ADR-0004, ADR-0005, ADR-0006]
superseded-by: null
deciders: []
---

# Separar a escrita remota segura da persistência local

## Context and Problem

A escrita atômica e o watcher locais não atravessam a fronteira SSH. Um upload direto pode deixar um
arquivo parcial; uma sessão também pode cair depois da substituição, antes da resposta.

## Decision Drivers

- Estender as salvaguardas da ADR-0004 sem alterar o contrato local.
- Editar in-place sem réplica sincronizada nem serviço permanente no host.
- Separar a confirmação do backup local da confirmação da escrita remota.

## Considered Options

### Option 1 — SFTP com leitura, temporário e rename

**Pros:** Protocolo próprio para arquivos. **Cons:** Releitura e troca são requisições separadas;
não há uma operação padrão de comparação condicional mais rename.

### Option 2 — Operação remota fechada de comparação e troca via SSH

**Pros:** Compara o alvo no host imediatamente antes da troca, sem daemon próprio. **Cons:** Exige
validar as primitivas do host e as falhas de transporte e de armazenamento.

### Option 3 — Montagem remota ou réplica sincronizada

**Pros:** Reusa APIs locais. **Cons:** Esconde diferenças de consistência e acrescenta montagem ou
reconciliação contínua fora do escopo.

## Decision

Usar a opção 2 para escrita. O backend vincula o documento à identidade verificada do destino,
conta, caminho permitido e SHA-256 da leitura. A gravação é síncrona e só segue após persistir os
bytes anteriores em snapshot privado no XDG local, conforme a ADR-0005. Os snapshots são segregados
por identidade remota, conta e caminho, sem colisão com os locais.

A operação remota é fixa e recebe conteúdo como dados. No host, prepara um temporário no diretório
do alvo, revalida o caminho, a identidade do arquivo e seu conteúdo antes da troca, e usa
substituição atômica preservando as proteções de acesso do arquivo. Não segue links simbólicos
abaixo do home resolvido e não degrada para upload direto, troca não atômica ou escrita como outro
usuário. A releitura no salvamento é a barreira de conflito remota; o watcher da ADR-0004 permanece
local.

Quando o transporte perde a confirmação da troca, não repete automaticamente a escrita. Uma nova
leitura autenticada do mesmo destino permite verificar o estado, sem tomar o backup ou o envio como
prova do resultado.

## Consequences

- Rename atômico evita arquivo parcial, mas não dá compare-and-swap contra editores externos não
  cooperativos. A corrida residual deve ser explicitada e não pode virar garantia de bloqueio
  absoluto de escritores concorrentes.
- Backups podem conter segredos: acesso restrito ao usuário do desktop, sem logs, telemetria ou
  publicação na biblioteca compartilhada.
- As primitivas e os requisitos do executor remoto precisam ser validados em Linux isolado antes de
  fechar a [SPEC-0005](../specs/0005-editar-configuracoes-globais-de-agentes-via-ssh.md).

## Confirmation

Executar o DoD da SPEC-0005 contra SSH isolado, incluindo conflito, identidade, permissões, backup,
substituição atômica e perda de confirmação sem repetição automática.
