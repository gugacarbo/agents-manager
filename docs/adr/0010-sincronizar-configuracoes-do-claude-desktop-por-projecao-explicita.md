---
status: proposed
date: 2026-11-27
builds-on: [ADR-0004, ADR-0005, ADR-0009]
superseded-by: null
deciders: []
---

# Sincronizar configurações do Claude Desktop por projeção explícita

## Context and Problem

Os perfis XDG isolam as sessões do Desktop. O usuário quer reaproveitar preferências de interface e
configurações de MCPs. Os arquivos do Desktop podem misturar preferências, estado privado e dados de
autenticação. A integração precisa controlar o que atravessa a fronteira entre perfis.

## Decision Drivers

- Preservar a autenticação administrada pelo Desktop em cada perfil.
- Permitir configurações próprias por destino.
- Reaproveitar a escrita segura do Manager com revisão e desfazimento.

## Considered Options

### Option 1 — Projetar campos de um perfil de referência por aplicação explícita

**Pros.** Controla quais dados circulam e permite preservar exceções por perfil.

**Cons.** Exige mapeamento verificado por formato e manutenção quando o Desktop muda.

### Option 2 — Compartilhar ou copiar arquivos inteiros entre perfis

**Pros.** Uma operação cobre todas as chaves conhecidas e futuras.

**Cons.** Acopla estado e credenciais, dificulta exceções e cria risco de sobrescrita pelo Desktop.

### Option 3 — Propagar mudanças automaticamente em ambas as direções

**Pros.** Os perfis convergem após edições feitas em qualquer um deles.

**Cons.** Exige resolver divergências concorrentes e pode propagar uma alteração de conta ou de
ambiente sem revisão.

## Decision

Usar uma projeção tipada e unidirecional de preferências verificadas, do perfil de referência para
os destinos selecionados, com aplicação explícita, edição cirúrgica e desfazimento por destino.

A projeção e a persistência de desfazimento recebem apenas os campos elegíveis. Para arquivos
mistos, o desfazimento guarda os valores anteriores dos campos modificados, incluindo a ausência de
um campo. A escrita em arquivo temporário privado serve à substituição atômica e não vira uma cópia
persistente do arquivo no armazenamento de backups. A recuperação opera sobre a mesma projeção e
respeita a edição posterior no destino.

## Consequences

Formatos ou campos desconhecidos ficam indisponíveis até serem verificados. O fluxo precisa
inspecionar a configuração de cada destino para preservar campos locais e detectar alterações após a
prévia. Uma falha de escrita em um destino não exige reverter os destinos concluídos.

## Confirmation

Executar o DoD da SPEC-0010 e inspecionar a projeção, o IPC e os registros de desfazimento com
fixtures que misturem preferências e dados privados fictícios. Confirmar que os registros guardam
somente os valores permitidos e que aplicar e desfazer preservam os campos fora da seleção.
