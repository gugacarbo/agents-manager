---
status: accepted
date: 2026-11-27
builds-on: [ADR-0004, ADR-0006, ADR-0009]
superseded-by: null
deciders: []
---

# Importar sessões do Claude Code entre perfis do Desktop

## Context and Problem

O isolamento dos perfis permite manter sessões de login independentes. Para retomar trabalho depois
de trocar a conta, o Desktop também precisa reconhecer a sessão do Claude Code no perfil de destino.
Sessões locais e sessões via SSH têm formas diferentes de armazenamento e retomada.

## Decision Drivers

- Permitir retomar uma conversa selecionada após a troca.
- Preservar a origem dos dados e a fronteira entre contas.
- Evitar conflito de escrita em sessões ativas e dependência da disponibilidade de todos os hosts.

## Considered Options

### Option 1 — Importação seletiva de entradas de sessões

**Pros.** Permite verificar a origem e reaproveitar a transcrição no mesmo local quando acessível.

**Cons.** Exige verificar o formato dos índices do Desktop e a retomada para cada tipo de sessão.

### Option 2 — Compartilhar o diretório de sessões do Desktop

**Pros.** Reduz a quantidade de arquivos que o Manager precisa interpretar.

**Cons.** Acopla os perfis, expõe histórico mais amplo e dificulta preservar exclusões e edições
locais.

## Decision

Importar entradas selecionadas para o índice do perfil de destino, reaproveitando a transcrição
local quando acessível ou copiando somente a transcrição local necessária quando o armazenamento for
separado, e manter transcrições SSH no host de origem.

A importação usa a identidade da sessão e sua origem para reconhecer entradas existentes. O
transporte SSH existente fornece descoberta de leitura nos ambientes cadastrados.

## Consequences

A importação depende da compatibilidade do formato e do acesso ao diretório da sessão. Falhas em um
host deixam as demais origens disponíveis e preservam as referências já existentes. A prova de
compatibilidade exige retomar a sessão pelo Desktop.

## Confirmation

Executar o DoD da SPEC-0009. Verificar com duas contas que uma sessão local e outra via SSH podem
ser retomadas na conta de destino, e inspecionar a ausência de cópias de transcrições SSH e dados de
autenticação nos arquivos gerenciados.
