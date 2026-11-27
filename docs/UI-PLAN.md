# Agents Manager — proposta de interface

**Direção:** um console desktop para localizar configurações, não um dashboard de métricas. A
primeira tela responde: **quais configurações foram detectadas e onde ficam?** A demonstração de
diff fica acessível, mas não se confunde com uma operação real.

**Base atual:** o Tauri/Linux detecta configurações de Claude Code, Codex e Antigravity. A versão
web isolada usa dados de fallback. A rota do editor usa JSON de exemplo em memória; não abre, grava
nem sincroniza arquivos reais. Os estados abaixo são exemplos, não resultados de uma detecção nesta
máquina.

## 1. Inventário local — `/`

**1200 × 800** · tamanho padrão da janela Tauri. Uma barra superior compacta, seguida de uma área
principal que usa a largura disponível. O caminho fica visível na própria linha, sem abrir detalhes.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Agents Manager     [Agents]   Diff demo                  [Search  Ctrl+K] │
├──────────────────────────────────────────────────────────────────────────┤
│ Local agents                                                   [Refresh] │
│ Find configuration folders for supported coding agents.                  │
│                                                                          │
│ [ Search by agent or path                   ] [All statuses ▾]           │
│ 2 of 3 configurations detected                                           │
│ ┌───────────────────┬────────────────┬─────────────────────────────────┐ │
│ │ Agent             │ Detection      │ Configuration folder            │ │
│ ├───────────────────┼────────────────┼─────────────────────────────────┤ │
│ │ Claude Code       │ ● Detected     │ ~/.claude                       │ │
│ │ Codex             │ ● Detected     │ ~/.codex                        │ │
│ │ Antigravity       │ ○ Not detected │ —                               │ │
│ └───────────────────┴────────────────┴─────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

**900 × 600** · alvo compacto para validação, não largura mínima oficial. A mesma lista passa a
mostrar nome e estado na primeira linha, caminho na segunda. A ordem dos agentes e dos controles não
muda.

```text
┌───────────────────────────────────────────────────────┐
│ Agents Manager   [Agents]  Diff demo  [Search Ctrl+K]  │
├───────────────────────────────────────────────────────┤
│ Local agents                               [Refresh]  │
│ [ Search by agent or path       ] [All statuses ▾]    │
│ 2 of 3 configurations detected                        │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Claude Code                            ● Detected │ │
│ │ ~/.claude                                         │ │
│ ├───────────────────────────────────────────────────┤ │
│ │ Codex                                  ● Detected │ │
│ │ ~/.codex                                          │ │
│ ├───────────────────────────────────────────────────┤ │
│ │ Antigravity                         ○ Not detected │ │
│ │ Configuration folder unavailable                  │ │
│ └───────────────────────────────────────────────────┘ │
└───────────────────────────────────────────────────────┘
```

### Estados e ações

| Situação                       | O que mostrar e fazer                                                                                                                                                                                          |
| ------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Primeira consulta              | `Checking local configuration folders…` como estado anunciado; reservar o espaço da lista.                                                                                                                     |
| Atualização                    | Manter os dados anteriores e indicar `Refreshing…`. Evitar pedidos duplicados. Comunicar sucesso só depois da resposta.                                                                                        |
| Falha na primeira consulta     | `Could not check local configuration folders` e botão `Try again` na área da lista; não mostrar contagem zero.                                                                                                 |
| Falha ao atualizar             | Manter a última lista, identificada como resultado anterior, e mostrar `Could not refresh` com `Try again`.                                                                                                    |
| Nenhuma configuração detectada | Manter os agentes suportados na lista como `Not detected`. Mostrar `0 of 3 configurations detected`; o estado não significa que o agente está desinstalado.                                                    |
| Filtro sem resultados          | `No agents match your filters` e `Clear filters`; não alterar o total da detecção.                                                                                                                             |
| Caminho indisponível ou longo  | Não inventar um caminho. Quando disponível, renderizar texto selecionável com quebra de linha; se abreviar o diretório pessoal como `~`, manter um meio de consultar o caminho completo sem depender de hover. |

Busca por nome ou caminho e filtro `All statuses / Detected / Not detected` restritos a esta tela. A
contagem resume a detecção geral; quando houver filtro ativo, mostrar também quantas linhas
correspondem. Linhas do inventário não parecem links enquanto não existir uma tela de detalhe.

## 2. Direção visual e navegação

- **Cor:** partir do fundo azulado `#020617` e superfície `#0F172A` atuais, com `#F8FAFC` para texto
  principal e `#94A3B8` para texto secundário. Propor azul-claro (`#60A5FA`) para foco/seleção.
  Verde e vermelho têm função semântica, acompanhados de ícone e texto; `Not detected` é neutro, não
  erro. Validar cores compostas, estados interativos e contraste dos separadores que definem
  controles na implementação.
- **Tipografia e ritmo:** fonte de sistema já usada no app, monospace apenas para caminhos, código e
  atalhos. Referência: título de tela 24 px, texto de interface 14 px, metadados 13 px e espaçamento
  em múltiplos de 4/8 px. Bordas discretas e sem sombras decorativas. Manter Lucide; não adicionar
  biblioteca nem depender de fontes remotas.
- **Navegação:** barra superior com `Agents`, `Diff demo` e botão `Search` com `Ctrl+K`. O botão
  abre a paleta de comandos; o campo `Search by agent or path` só filtra a lista. A paleta deve
  distinguir `Go to Agents`, `Open Diff demo` e `Refresh agents`. Não usar barra lateral até haver
  áreas suficientes. Textos em inglês preservam a convenção atual, sem decidir localização.

## 3. Demonstração — `/editor`

Em janela larga, editor e diff de exemplo lado a lado; ambos com rolagem própria. O diff compara o
rascunho com o exemplo original. O divisor pode ser ajustável se também puder ser operado por
teclado; divisão padrão equilibrada. Não existe botão de gravação nem `Preview Sync`.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Agents Manager      Agents   [Diff demo]                 [Search  Ctrl+K] │
├──────────────────────────────────────────────────────────────────────────┤
│ Diff demo                                                                │
│ Sample JSON only. No local files are changed.             [Reset sample] │
│ ┌───────────────────────────────┬──────────────────────────────────────┐ │
│ │ Sample editor                 │ Changes from original               │ │
│ │ {                             │ - "path": "/workspace"              │ │
│ │   "path": "/home/demo"        │ + "path": "/home/demo"              │ │
│ │ }                             │                                      │ │
│ └───────────────────────────────┴──────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────┘
```

Em janela compacta, trocar as duas colunas por abas `Sample editor` e `Changes from original`.
Preservar o rascunho ao alternar abas. `Reset sample` só deve aparecer se restaurar o exemplo de
fato; fica indisponível se nada mudou. Se o rascunho não persistir ao sair da rota, deixar isso
explícito na demo. Adições e remoções têm sinais `+` e `−` além da cor.

### Fluxo futuro — fora da primeira entrega

```text
Agente → Global / Project → MCPs / Skills / Rules / Settings
       → Arquivo e caminho nativo → Edição → Revisão do diff → Gravar
```

Manter agente, escopo e caminho visíveis no cabeçalho do editor e na revisão. Não apresentar
`Gravar` como disponível até existir suporte real e validação. Resolução de mudanças externas,
tratamento de segredos e recuperação precisam de decisão explícita e suporte do backend antes de
virar promessa da interface. Biblioteca compartilhada e sincronização entre agentes são fluxos
separados e opcionais, ainda não comprometidos.

## 4. Revisão e aceite

| Cenário                                    | Resultado esperado                                                                                                                                                                                                           |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Abrir o app e localizar uma configuração   | Agente, estado e caminho aparecem na primeira tela, sem depender de abrir cartão nem de usar a paleta.                                                                                                                       |
| Diferenciar falha, ausência e filtro vazio | Falha oferece nova tentativa; ausência mantém os agentes suportados; filtro vazio oferece limpar os filtros.                                                                                                                 |
| Abrir a demonstração e modificar o JSON    | Aviso de modo de exemplo fica visível; mudar o texto atualiza a comparação, sem ação que sugira gravar. Na compacta, alternar abas não perde o rascunho.                                                                     |
| Operar com teclado                         | Ordem de foco acompanha a interface; foco visível e não escondido por barras; `Ctrl+K` abre a paleta, `Esc` fecha e devolve o foco ao controle anterior ao cancelar. Nenhum fluxo depende de arrastar ou de hover.           |
| Testar janela e legibilidade               | Conferir 1200 × 800 e 900 × 600 com caminhos longos e zoom; não exigir rolagem horizontal da janela. Contraste mínimo proposto de 4,5:1 para texto normal, inclusive metadados. Respeitar preferência de movimento reduzido. |

A revisão destes wireframes é documental, não um teste de usabilidade ou de acessibilidade do app.
Em uma implementação futura: Vitest para estados e atualização, Playwright para teclado e os dois
tamanhos de janela, e teste manual no Tauri/Linux para validar a detecção real e o comportamento no
WebView.

**Referências:** `PRODUCT.md`, `src/router.tsx`, `src/index.css`, `src-tauri/tauri.conf.json` e a
referência local `ui-ux-pro-max` (perfil Developer Tool / IDE, guias de teclado, foco, estados
vazios e movimento reduzido). O perfil serviu de ponto de partida; a proposta foi ajustada à janela
desktop e à capacidade atual do app.
