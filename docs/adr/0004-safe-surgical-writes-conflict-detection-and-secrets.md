---
status: accepted
date: 2026-11-26
builds-on: [ADR-0001, ADR-0002]
superseded-by: null
deciders: [gugacarbo]
---

# Exigir escrita cirúrgica, detecção de conflitos externos e mascaramento de segredos

## Context and Problem

O `agents-manager` modifica arquivos reais que controlam o ambiente de trabalho diário do
desenvolvedor e que também podem ser alterados por editores externos ou pelos próprios agentes em
runtime. Além disso, arquivos de configuração de MCPs frequentemente contêm chaves e tokens literais
em variáveis de ambiente (`env`), trazendo risco de corrupção de dados, perda de comentários/campos
desconhecidos e exposição acidental de credenciais.

## Decision Drivers

- Nunca corromper arquivos reais em caso de falha parcial durante a gravação.
- Preservar comentários, formatação e chaves desconhecidas de ferramentas em evolução constante
  (TOML e JSON).
- Evitar perda silenciosa de edições feitas fora do aplicativo enquanto o arquivo estava aberto.
- Proteger credenciais contra exposição visual acidental e impedir que valores literais sejam
  copiados sem aviso para a biblioteca compartilhada.

## Considered Options

### Option 1 — Escrita cirúrgica com escrita atômica, preservação estrutural, SHA-256 + file watcher passivo e mascaramento de segredos

**Pros:** Combina gravação atômica (arquivo temporário no mesmo diretório + rename atômico), backup
prévio, edição estrutural preservando comentários/chaves desconhecidas (`toml_edit` em TOML e merge
cirúrgico em JSON), detecção de edição externa por hash SHA-256 + file watcher passivo (`notify`)
com resolução explícita pelo usuário, e mascaramento visual de prováveis segredos com alerta ao
promover valores literais para a biblioteca compartilhada (incentivando `${VAR}`).

**Cons:** Aumenta a complexidade inicial do motor em Rust em comparação com serialização simples via
`serde`.

### Option 2 — Serialização padrão (serde) com checagem de hash apenas ao salvar

**Pros:** Implementação mais rápida e direta.

**Cons:** Reformata arquivos inteiros, descarta comentários de TOML e só avisa sobre alterações
externas no momento tardio do salvamento.

### Option 3 — Modo somente leitura para Settings e Regras na v1

**Pros:** Reduz risco de sobrescrita em arquivos complexos.

**Cons:** Quebra o objetivo central de editar e gerenciar configurações em um único lugar.

## Decision

Adotar **escrita cirúrgica com salvaguardas completas**: toda mutação em arquivos reais deve usar
escrita atômica com backup automático prévio, preservação estrutural de comentários e campos
desconhecidos, detecção de edição externa via **hash SHA-256 + file watcher passivo (`notify`)**
exigindo escolha explícita do usuário (_recarregar_ ou _sobrescrever com preview de diff_), e
**mascaramento visual de segredos** na UI com alerta ao promover valores literais para a biblioteca
compartilhada.

## Consequences

- **Positivas:** Elimina corrupção por escrita parcial, preserva customizações manuais nos arquivos
  originais e protege credenciais contra cópia inadvertida.
- **Negativas:** Exige modelar estado de conflito, checksum de leitura/escrita e eventos de watcher
  no ciclo de edição entre Rust e React.
- **Obrigatório:** Proibir `unwrap_used`/`expect_used` em código de produção do workspace Rust;
  retornar erros tipados (`CoreError`) em qualquer falha de I/O, parsing ou conflito de versão.
- **Proibido:** Sobrescrever silenciosamente um arquivo cujo hash atual no disco divirja do último
  hash lido ou gravado pelo app; gravar diretamente sobre o arquivo alvo sem escrita atômica e
  backup prévio.

## Confirmation

```bash
rg -n "unwrap_used = \"deny\"" Cargo.toml && cargo clippy --workspace --all-targets -- -D warnings
```

## Notes

- Lints de segurança e prevenção de _panic_ já ativos no workspace em `Cargo.toml`.
- Retenção e localização dos backups são definidas em `ADR-0005`.
