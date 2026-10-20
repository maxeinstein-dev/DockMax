---
name: DockMax
description: Dock para Windows 11 com silhueta de Big Sur, vidro simulado e ladrilhos de cerâmica esmaltada em tokens trocáveis.
colors:
  cobalto-light: "#2f5bd3"
  cobalto-dark: "#7c9cff"
  cobalto-ink-light: "#1f45b3"
  cobalto-ink-dark: "#b0c2ff"
  cobalto-soft-light: "rgb(47 91 211 / .13)"
  cobalto-soft-dark: "rgb(124 156 255 / .17)"
  perigo-light: "hsl(10 62% 40%)"
  perigo-dark: "hsl(10 86% 72%)"
  perigo-ink-light: "#ffffff"
  perigo-ink-dark: "#1d1e22"
  perigo-soft-light: "hsl(10 62% 40% / .1)"
  perigo-soft-dark: "hsl(10 86% 72% / .14)"
  porcelana-surface: "#f7f7f9"
  grafite-surface: "#26272c"
  ink-light: "#1d1e22"
  ink-dark: "#ececf0"
  ink-2-light: "#5b5f69"
  ink-2-dark: "#a5a9b3"
  hair-light: "rgb(0 0 0 / .09)"
  hair-dark: "rgb(255 255 255 / .1)"
  hi-light: "rgb(255 255 255 / .8)"
  hi-dark: "rgb(255 255 255 / .16)"
  tip-bg-light: "rgb(252 252 253 / .97)"
  tip-bg-dark: "rgb(50 51 57 / .97)"
  tip-ink-light: "#1d1e22"
  tip-ink-dark: "#f2f2f5"
  track-light: "rgb(29 30 34 / .055)"
  track-dark: "rgb(255 255 255 / .06)"
  kbd-light: "rgb(29 30 34 / .07)"
  kbd-dark: "rgb(255 255 255 / .09)"
  field-light: "rgb(255 255 255 / .85)"
  field-dark: "rgb(255 255 255 / .045)"
  field-line-light: "rgb(29 30 34 / .5)"
  field-line-dark: "rgb(255 255 255 / .4)"
  pg-fill-light: "hsl(84 26% 38%)"
  pg-fill-dark: "hsl(84 30% 62%)"
  pg-track-light: "rgb(29 30 34 / .08)"
  pg-track-dark: "rgb(255 255 255 / .09)"
  glifo: "#ffffff"
  salvia: "hsl(150 22% 44%)"
  ocre: "hsl(38 48% 40%)"
  petroleo: "hsl(192 26% 40%)"
  musgo: "hsl(84 22% 38%)"
  argila: "hsl(16 40% 44%)"
  ameixa: "hsl(318 20% 43%)"
  tinta: "hsl(222 14% 34%)"
  ardosia: "hsl(214 22% 46%)"
typography:
  clock-time:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 600
    lineHeight: 1.05
    letterSpacing: "-.02em"
    fontFeature: "tnum"
  clock-seconds:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "22px"
    fontWeight: 600
    letterSpacing: "0"
    fontFeature: "tnum"
  clock-ampm:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    letterSpacing: "0"
  tile-time:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "calc(var(--icon) * .28)"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "-.01em"
    fontFeature: "tnum"
  tile-pct:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "calc(var(--icon) * .23)"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "-.02em"
    fontFeature: "tnum"
  title:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    lineHeight: 1.3
  title-about:
    fontFamily: "Segoe UI Variable Display, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "17px"
    fontWeight: 600
    lineHeight: 1.3
  label:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "12.5px"
    fontWeight: 400
  label-strong:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "12.5px"
    fontWeight: 600
  note-body:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "12.5px"
    fontWeight: 400
    lineHeight: 1.5
  caption:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
  kbd:
    fontFamily: "Segoe UI Variable Text, Segoe UI Variable, Segoe UI, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1
rounded:
  bar-s: "18px"
  bar-m: "20px"
  bar-l: "24px"
  panel: "8px"
  track: "6px"
  inset: "6px"
  segment: "4px"
  control: "4px"
  kbd: "4px"
  meter: "3px"
  tile: "23%"
  pill: "999px"
  dot: "50%"
spacing:
  icon-s: "40px"
  icon-m: "48px"
  icon-l: "64px"
  pad-s: "6px"
  pad-m: "7px"
  pad-l: "9px"
  gap-s: "6px"
  gap-m: "7px"
  gap-l: "9px"
  edge-offset: "8px"
  panel-gap: "14px"
  panel-padding: "16px 16px 12px"
  panel-clock: "300px"
  panel-progress: "320px"
  panel-note: "340px"
  note-field: "210px"
  items-list-max: "268px"
  icon-button: "26px"
  tile-mini: "26px"
  tile-mini-add: "18px"
  badge: "10px"
  meter: "6px"
components:
  dock-bar-light:
    backgroundColor: "{colors.porcelana-surface}"
    rounded: "{rounded.bar-m}"
    padding: "{spacing.pad-m}"
  dock-bar-dark:
    backgroundColor: "{colors.grafite-surface}"
    rounded: "{rounded.bar-m}"
    padding: "{spacing.pad-m}"
  dock-tile:
    textColor: "{colors.glifo}"
    rounded: "{rounded.tile}"
    size: "{spacing.icon-m}"
  live-tile-clock:
    backgroundColor: "{colors.petroleo}"
    textColor: "{colors.glifo}"
    typography: "{typography.tile-time}"
    rounded: "{rounded.tile}"
    size: "{spacing.icon-m}"
  live-tile-progress:
    backgroundColor: "{colors.musgo}"
    textColor: "{colors.glifo}"
    typography: "{typography.tile-pct}"
    rounded: "{rounded.tile}"
    size: "{spacing.icon-m}"
  tile-badge-light:
    backgroundColor: "{colors.perigo-light}"
    rounded: "{rounded.dot}"
    size: "{spacing.badge}"
  tile-badge-dark:
    backgroundColor: "{colors.perigo-dark}"
    rounded: "{rounded.dot}"
    size: "{spacing.badge}"
  tile-mini:
    textColor: "{colors.glifo}"
    rounded: "{rounded.tile}"
    size: "{spacing.tile-mini}"
  open-indicator:
    backgroundColor: "{colors.cobalto-light}"
    rounded: "{rounded.dot}"
    size: "4px"
  tooltip-light:
    backgroundColor: "{colors.tip-bg-light}"
    textColor: "{colors.tip-ink-light}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.pill}"
    padding: "4px 10px"
  tooltip-dark:
    backgroundColor: "{colors.tip-bg-dark}"
    textColor: "{colors.tip-ink-dark}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.pill}"
    padding: "4px 10px"
  panel-light:
    backgroundColor: "{colors.porcelana-surface}"
    textColor: "{colors.ink-light}"
    rounded: "{rounded.panel}"
    padding: "{spacing.panel-padding}"
    width: "320px"
  panel-dark:
    backgroundColor: "{colors.grafite-surface}"
    textColor: "{colors.ink-dark}"
    rounded: "{rounded.panel}"
    padding: "{spacing.panel-padding}"
    width: "320px"
  panel-clock:
    backgroundColor: "{colors.porcelana-surface}"
    textColor: "{colors.ink-light}"
    rounded: "{rounded.panel}"
    padding: "{spacing.panel-padding}"
    width: "{spacing.panel-clock}"
  panel-progress:
    backgroundColor: "{colors.porcelana-surface}"
    textColor: "{colors.ink-light}"
    rounded: "{rounded.panel}"
    padding: "{spacing.panel-padding}"
    width: "{spacing.panel-progress}"
  panel-note:
    backgroundColor: "{colors.porcelana-surface}"
    textColor: "{colors.ink-light}"
    rounded: "{rounded.panel}"
    padding: "{spacing.panel-padding}"
    width: "{spacing.panel-note}"
  segmented-track-light:
    backgroundColor: "{colors.track-light}"
    rounded: "{rounded.track}"
    padding: "2px"
  segment-light:
    textColor: "{colors.ink-light}"
    typography: "{typography.label}"
    rounded: "{rounded.segment}"
    padding: "6px"
  segment-light-selected:
    backgroundColor: "{colors.cobalto-soft-light}"
    textColor: "{colors.cobalto-ink-light}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.segment}"
    padding: "6px"
  segment-dark-selected:
    backgroundColor: "{colors.cobalto-soft-dark}"
    textColor: "{colors.cobalto-ink-dark}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.segment}"
    padding: "6px"
  field-light:
    backgroundColor: "{colors.field-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 8px"
  field-dark:
    backgroundColor: "{colors.field-dark}"
    textColor: "{colors.ink-dark}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 8px"
  note-field:
    backgroundColor: "{colors.field-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.note-body}"
    rounded: "{rounded.control}"
    padding: "8px 10px"
    height: "{spacing.note-field}"
  button-quiet:
    textColor: "{colors.ink-2-light}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "4px 8px"
  button-icon:
    textColor: "{colors.ink-2-light}"
    rounded: "{rounded.control}"
    size: "{spacing.icon-button}"
  button-secondary-light:
    backgroundColor: "{colors.track-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 10px"
  button-danger-light:
    backgroundColor: "{colors.perigo-light}"
    textColor: "{colors.perigo-ink-light}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.control}"
    padding: "6px 12px"
  button-danger-dark:
    backgroundColor: "{colors.perigo-dark}"
    textColor: "{colors.perigo-ink-dark}"
    typography: "{typography.label-strong}"
    rounded: "{rounded.control}"
    padding: "6px 12px"
  button-remove-light:
    textColor: "{colors.perigo-light}"
    typography: "{typography.label}"
    rounded: "{rounded.control}"
    padding: "6px 8px"
  nav-row-light:
    backgroundColor: "{colors.track-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.label}"
    rounded: "{rounded.inset}"
    padding: "8px 8px 8px 10px"
  notice-light:
    backgroundColor: "{colors.track-light}"
    textColor: "{colors.ink-2-light}"
    typography: "{typography.label}"
    rounded: "{rounded.inset}"
    padding: "10px 12px"
  confirm-light:
    backgroundColor: "{colors.perigo-soft-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.label}"
    rounded: "{rounded.inset}"
    padding: "12px"
  progress-meter-light:
    backgroundColor: "{colors.pg-track-light}"
    rounded: "{rounded.meter}"
    height: "{spacing.meter}"
  progress-meter-fill-light:
    backgroundColor: "{colors.pg-fill-light}"
    rounded: "{rounded.meter}"
    height: "{spacing.meter}"
  kbd-light:
    backgroundColor: "{colors.kbd-light}"
    textColor: "{colors.ink-light}"
    typography: "{typography.kbd}"
    rounded: "{rounded.kbd}"
    padding: "3px 6px"
---

# Design System: DockMax

## Overview

**Creative North Star: "Big Sur, nascido no Windows"**

O DockMax é o dock do macOS que o autor gosta, refeito como peça do Windows 11: a silhueta e a ampliação em degraus do Big Sur, com materiais e cantos que conversam com as janelas do Windows. A barra é uma placa de vidro simulado, "porcelana fosca" no tema claro e "grafite fosco" no escuro, com uma borda interna de 1 px que pega luz no lado oposto à borda da tela e uma sombra macia deslocada para fora. Sobre ela, os ícones são ladrilhos de cerâmica esmaltada numa paleta própria de tons terrosos e frios dessaturados, cada um com um glifo de traço branco.

O sistema é discreto no uso diário e caprichado no detalhe: a identidade está no acabamento (esmalte, borda de luz, sombra), não no volume. Há um único destaque, azul-cobalto, e ele só aparece para dizer algo: foco, seleção, painel aberto. A ampliação é contida (1,4 no item, 1,15 nos vizinhos), congela enquanto um painel está aberto e some com "reduzir movimento".

Recusas confirmadas: a barra escura genérica de launcher e o arco-íris de ícones da Apple. Não há desfoque real do fundo (ADR 0006): o vidro é simulado com uma camada tingida, e a legibilidade vem da opacidade. Toda a identidade vive em tokens CSS estáticos, porque a personalização é requisito do produto e o app roda com CSP estrita.

Este sistema registra a variante **B1 "Big Sur"**, escolhida pelo usuário no mock de decisão `docs/design/dock-mock.html` (estados reproduzíveis por parâmetros de URL: `?variant=b1&theme=dark&size=l&opacity=solid&edge=top&panel=about&forced=focus`). A variante B2 "Prateleira" (ícones sobre prateleira de vidro em perspectiva, ampliação parabólica contínua, rótulo retangular, reflexo curto) foi avaliada e rejeitada; seus tokens não fazem parte do sistema.

**Extensão da Fase 2 (aprovada em 2026-10-16).** Os widgets são ladrilhos do próprio dock que mostram o valor vivo no esmalte (a hora, o anel do dia) e abrem um flyout do Windows 11 com o detalhe e os ajustes na mesma folha. O mundo B1 não muda; entram um esmalte novo (musgo), o campo de texto do Windows 11, um token de perigo argila-vermelho só para remoção, erro e selo, e os componentes de painel que os widgets pedem. Recusas confirmadas: a grade de cards de widget e o painel lateral de widgets do Windows. Mock de decisão: `docs/design/widgets-mock.html` (estados por URL, por exemplo `?open=note&view=confirm&theme=dark&time=18:42`).

**Key Characteristics:**
- Placa de vidro simulado com cantos grandes, proporcionais ao tamanho (18/20/24 px).
- Ladrilhos em superelipse com esmalte de dois tons da mesma matiz e glifo de traço branco 1,75.
- Destaque único azul-cobalto, reservado a foco, seleção e ponto de painel aberto.
- Ampliação em degraus que empurra os vizinhos, com rótulo em pílula acima do ícone.
- Painel sempre opaco, com cantos de 8 px como as janelas e flyouts do Windows 11.
- Dois temas por `[data-mode]`, duas opacidades do dock, três tamanhos, duas bordas: tudo troca de valor, nunca de componente.
- Ícones vivos: o Relógio e o Progresso mostram o valor em branco no próprio esmalte, atualizado uma vez por minuto.
- Uma folha só por widget: detalhe e Ajustes no mesmo painel, com Voltar; confirmações e avisos dentro da folha, nunca em modal.

## Colors

Neutros frios quase sem matiz, um azul-cobalto contido e uma paleta de esmaltes terrosos e frios, todos dessaturados para não cansar ao longo do dia; um argila-vermelho de perigo, raro, só para o que destrói ou falha.

### Primary
- **Azul-cobalto** (`cobalto-light` / `cobalto-dark`, token `--accent`): contorno de foco do ladrilho e dos segmentos (2 px), anel interno do segmento selecionado (a 40%) e ponto de painel aberto. No escuro, sobe para um cobalto claro para manter contraste sobre o grafite. Na Fase 2 também marca o foco do campo de texto (fio inferior e cursor) e a caixa de seleção marcada.
- **Tinta de cobalto** (`cobalto-ink-light` / `cobalto-ink-dark`, token `--accent-ink`): texto do segmento selecionado (e, a partir da Fase 3, o link do painel Sobre).
- **Véu de cobalto** (`cobalto-soft-light` / `cobalto-soft-dark`, token `--accent-soft`): fundo do segmento selecionado e da seleção de texto no painel.

### Secondary (perigo)
- **Argila-vermelho** (`perigo-light` / `perigo-dark`, token `--danger`): fundo do botão "Remover nota", texto do "Remover do dock", estado de erro do salvamento da nota, selo de aviso no ladrilho, ponto de aviso na linha "Itens do dock" e ícone do aviso inline. No escuro, sobe para um salmão claro para manter contraste sobre o grafite.
- **Tinta sobre perigo** (`perigo-ink-light` / `perigo-ink-dark`, token `--danger-ink`): texto do botão de perigo (branco no claro, tinta no escuro).
- **Véu de perigo** (`perigo-soft-light` / `perigo-soft-dark`, token `--danger-soft`): fundo da confirmação inline e hover do "Remover do dock".

### Tertiary (esmaltes dos ladrilhos)
Classes estáticas `.t-*`, definidas por matiz, saturação e luminosidade (`--h`, `--sat`, `--l`), para que o esmalte seja calculado em CSS sem estilo inline.
- **Tinta** (`tinta`): Configurações. O tom mais sóbrio, para a ferramenta.
- **Ardósia** (`ardosia`): Sobre.
- **Petróleo** (`petroleo`, `.t-clock`): Relógio.
- **Musgo** (`musgo`, `.t-progress`): Progresso do tempo. Novo na Fase 2, para o Progresso não se confundir com Arquivos (sálvia).
- **Ocre** (`ocre`, `.t-notes`): Nota.
- **Sálvia** (`salvia`), **Argila** (`argila`), **Ameixa** (`ameixa`): reservados aos itens da Fase 3 (Arquivos, Calendário, Música). Enquanto forem ilustrativos, entram com a saturação multiplicada por 0,55 (`--sat-k`).
- **Glifo** (`glifo`): o traço branco sobre todo ladrilho, e os números e ponteiros dos ícones vivos.

### Neutral
- **Porcelana fosca** (`porcelana-surface`, token `--surface-rgb: 247 247 249`): superfície da barra e do painel no tema claro.
- **Grafite fosco** (`grafite-surface`, token `--surface-rgb: 38 39 44`): superfície da barra e do painel no tema escuro.
- **Tinta** (`ink-*`, `--ink`): texto principal do painel e cor herdada do dock.
- **Tinta secundária** (`ink-2-*`, `--ink-2`): rótulos das linhas do painel, rodapé, chaves da lista do Sobre, segundos e AM/PM do Relógio, data, "faltam …" do Progresso.
- **Fio** (`hair-*`, `--hair`): anel externo de 1 px da barra, do painel e da pílula; separador; linha do rodapé; borda do campo; divisória da lista de itens.
- **Luz de borda** (`hi-*`, `--hi`): o 1 px interno que pega luz na barra e no painel.
- **Pílula** (`tip-bg-*`, `tip-ink-*`): fundo quase opaco (97%) e texto do rótulo.
- **Trilho** (`track-*`, `--track`): fundo do controle segmentado e hover do segmento; fundo do botão secundário, da linha de navegação e do aviso inline; hover dos botões quieto e de ícone.
- **Tecla** (`kbd-*`, `--kbd`): fundo da dica de teclado.
- **Campo** (`field-*`, `--field`) e **fio do campo** (`field-line-*`, `--field-line`): fundo e fio inferior do campo de texto do Windows 11.
- **Barra de progresso** (`pg-fill-*`, `--pg-fill`; `pg-track-*`, `--pg-track`): preenchimento musgo e trilho das barras do painel do Progresso. O preenchimento é da família do musgo, mas não o mesmo valor do esmalte (saturação 26% no claro, 30% e luminosidade 62% no escuro).

### Named Rules
**The One Cobalt Rule.** O cobalto existe só para foco, seleção e painel aberto. Nunca em ladrilho, superfície, texto corrido ou decoração. Se um elemento azul não responde a uma dessas três perguntas, está errado.

**The Danger Only Rule.** O perigo (`--danger`) existe só para três coisas: confirmar a remoção, mostrar o erro de salvamento e marcar o aviso (selo no ladrilho, ponto na linha "Itens do dock", ícone do aviso inline). Nunca decorativo, nunca em ladrilho, superfície ou título. Ele não substitui o cobalto: foco, seleção e painel aberto continuam cobalto, inclusive dentro da confirmação.

**The Own Glaze Rule.** Todo ladrilho sai da paleta própria, em classe estática. Nenhum ícone, cor de marca ou arquivo de tema de terceiros; um item novo ganha uma matiz nova no mesmo registro dessaturado (saturação 14 a 48%, luminosidade 34 a 46%), como o musgo (84 22% 38%) do Progresso.

**The Swappable Identity Rule.** Toda cor é um token por tema em `[data-mode="light"]` / `[data-mode="dark"]`. O destaque é um token só, para que a escolha de destaque pelo usuário entre depois sem retrabalho. Exceção nomeada: `--popup-ink`, a tinta das `<option>` da lista aberta de um `<select>`, tem um valor só, o da `--ink` clara, em todos os temas, porque o popup do WebView2 pinta fundo claro mesmo no tema escuro (ADR 0007).

## Typography

**Display Font:** Segoe UI Variable Display (com Segoe UI Variable, Segoe UI, system-ui)
**Body Font:** Segoe UI Variable Text (com Segoe UI Variable, Segoe UI, system-ui)

**Character:** A fonte do próprio Windows 11, em corpo pequeno e peso firme: o dock deve parecer parte do shell, não um site. Não há fontes web; a CSP estrita do app não as carrega e o mock também não.

### Hierarchy
- **Clock Time** (600, 40px, 1,05, −0,02em, Display, algarismos tabulares): hora no painel do Relógio.
- **Clock Seconds** (600, 22px, Display, tabular, `--ink-2`): segundos no painel do Relógio, quando o ajuste "Segundos no painel" está ligado; alinhados pela linha de base.
- **Clock AM/PM** (600, 15px, Display, `--ink-2`, 6 px à esquerda): marcador do formato 12 horas, só no painel.
- **Title About** (600, 17px, 1,3, Display): nome "DockMax" ao lado do ladrilho de 56 px no painel Sobre.
- **Title** (600, 15px, 1,3, Display): título do painel ("Configurações", nome do widget, "Ajustes do relógio", "Itens do dock"); corta com reticências numa linha.
- **Tile Time** (600, ícone × 0,28, 1, −0,01em, Display, tabular, branco): hora no ícone do Relógio digital (11,2/13,4/17,9 px nos três tamanhos).
- **Tile Pct** (600, ícone × 0,23, 1, −0,02em, Display, tabular, branco): percentual no anel do Progresso; o sinal `%` a 0,7 em e 85%. Some no Pequeno.
- **Label** (400, 12,5px): rótulos das linhas do painel, segmentos, linhas do Sobre, campo de texto, botões, aviso, data do Relógio, linhas do Progresso.
- **Label Strong** (600, 12,5px): texto da pílula, segmento selecionado, botão de perigo, percentual do Progresso no painel, nome na lista de itens e na linha de navegação.
- **Note Body** (400, 12,5px, 1,5): texto da Nota.
- **Caption** (400, 12px): rodapé do painel ("Salvo automaticamente", "Projeto independente"), "faltam …" do Progresso, subtítulo na lista de itens, contagem "3 de 12", dicas.
- **Kbd** (600, 11px, 1): dica de tecla (`Esc`).

### Named Rules
**The Shell Font Rule.** Só Segoe UI Variable, em pesos 400 e 600. Display apenas em títulos de painel e em números grandes (a hora do painel e os números dos ícones vivos); o resto é Text.

**The Tabular Number Rule.** Todo número que muda sozinho (hora, segundos, percentual, contagem, "faltam …") usa algarismos tabulares, para não tremer a cada minuto.

## Layout

O dock é centralizado na borda inferior (ou superior) do monitor, 8 px acima da área útil (`edge-offset`). A barra é uma linha flexível de itens alinhada à borda da tela, com altura `icon + 2 × pad` e o mesmo valor para `pad` e `gap` em cada tamanho:

| Tamanho | `--icon` | `--pad` | `--gap` | `--r-bar` |
|---|---|---|---|---|
| Pequeno (`s`) | 40px | 6px | 6px | 18px |
| Médio (`m`) | 48px | 7px | 7px | 20px |
| Grande (`l`) | 64px | 9px | 9px | 24px |

**Ampliação (B1, em degraus).** O item sob o mouse (ou com foco de teclado) escala para 1,4 e sobe 6 px; os vizinhos imediatos escalam para 1,15. A largura do item cresce junto (`width: icon × s`), então a ampliação **empurra os vizinhos** e a barra cresce na horizontal, em vez de sobrepor. O ladrilho cresce a partir da borda da tela (`transform-origin` na base, ou no topo com o dock em cima). Congela enquanto há painel aberto e é desligada com "reduzir movimento".

**Ordem do dock.** Widgets primeiro, na ordem de Configurações › Itens do dock (até 12), depois o separador, Configurações e Sobre.

**Painel.** Ancorado ao item: centrado nele, 8 px além da barra, limitado a 8 px das bordas da tela. Grade de uma coluna com 14 px entre linhas e padding 16/16/12 px. Larguras:

| Painel | Largura |
|---|---|
| Configurações (e Itens do dock) | 320px |
| Sobre | 300px |
| Relógio | 300px |
| Progresso do tempo | 320px |
| Nota | 340px |

O painel precisa caber acima do dock na área útil em qualquer estado: por isso a lista de itens rola acima de 268 px e o campo da Nota tem altura fixa de 210 px.

**Restrições para os PRs 2 e 3.**
- A janela do dock precisa de folga lateral além da de cima, porque a barra alarga ao ampliar.
- No tamanho Grande, o item ampliado chega a ~105 px acima da base (64 × 1,4 + 6 + 9), contra os 104 px da janela atual (barra de 64 + folga de 40). O PR 2 precisa aumentar a folga conforme o tamanho ou limitar a ampliação no Grande.
- Tokens em CSS estático. Escalas por item via CSSOM (`style.setProperty("--s", …)`) são compatíveis com a CSP; atributos `style` injetados no HTML não são.

## Elevation & Depth

Híbrido de vidro simulado e sombra ambiente. Não existe desfoque do fundo: a profundidade vem de três camadas empilhadas na mesma `box-shadow` (luz de borda interna, anel de fio de 1 px, sombra macia deslocada para fora da tela) e da opacidade da superfície. O dock tem duas opacidades (`--surface-a`: 0,8 Translúcido, 1 Sólido); o painel é sempre opaco. Dentro do painel tudo é plano: aviso, confirmação, linha de navegação e botões se separam por tom (`--track`, `--danger-soft`), nunca por sombra.

### Shadow Vocabulary
- **Sombra da superfície, claro** (`--shadow: 0 1px 2px rgb(16 20 32 / .10), 0 14px 32px -12px rgb(16 20 32 / .42)`): barra.
- **Sombra da superfície, escuro** (`--shadow: 0 1px 2px rgb(0 0 0 / .32), 0 16px 36px -12px rgb(0 0 0 / .7)`): barra.
- **Sombra do painel, claro** (`--shadow-panel: 0 1px 2px rgb(16 20 32 / .10), 0 2px 8px -2px rgb(16 20 32 / .24)`) e **escuro** (`--shadow-panel: 0 1px 2px rgb(0 0 0 / .32), 0 2px 8px -2px rgb(0 0 0 / .5)`): a mesma sombra em versão curta, que cabe nos 8 px de margem transparente da janela do painel (a janela bloqueia cliques e não pode cobrir a barra do dock, ADR 0006).
- **Pilha da barra** (`box-shadow: inset 0 1px 0 var(--hi), 0 0 0 1px var(--hair), var(--shadow)`): com o dock em cima, a luz interna passa para a borda de baixo (`inset 0 -1px 0`). O painel usa a mesma pilha com a sombra do painel no lugar de `--shadow`.
- **Sombra da pílula** (`box-shadow: 0 0 0 1px var(--hair), 0 6px 16px -6px rgb(0 0 0 / .35)`).
- **Sombra do ladrilho** (`filter: drop-shadow(0 1px 1px rgb(0 0 0 / .18)) drop-shadow(0 5px 6px rgb(0 0 0 / .16))`): em `filter` no invólucro sem máscara, para seguir a silhueta da superelipse. O ladrilho mini da lista usa só `drop-shadow(0 1px 1px rgb(0 0 0 / .2))`; o dos botões de adicionar, nenhuma.
- **Bordas do esmalte** (`box-shadow: inset 0 1px 0 rgb(255 255 255 / .32), inset 0 -1px 0 rgb(0 0 0 / .16)`).
- **Sombra dos números vivos** (`text-shadow: 0 1px 1px rgb(0 0 0 / .18)`): a hora do Relógio digital no esmalte.
- **Anel do selo** (`box-shadow: 0 0 0 2px rgb(var(--surface-rgb))`): recorta o selo do ladrilho com a cor da superfície.

### Named Rules
**The Opaque Panel Rule.** O painel é sempre opaco (`rgb(var(--surface-rgb))`), qualquer que seja a opacidade do dock. Sem desfoque, texto translúcido sobre o Chrome de trás não se lê.

**The Light Edge Rule.** A luz de borda de 1 px fica sempre no lado oposto à borda da tela: em cima com o dock embaixo, embaixo com o dock em cima.

## Shapes

Três famílias de canto. A barra tem cantos grandes que crescem com o tamanho (18/20/24 px), à maneira do Big Sur. O painel usa 8 px, os cantos das janelas e flyouts do Windows 11; dentro dele, 6 px para os blocos embutidos (trilho do segmentado, aviso, confirmação, linha de navegação) e 4 px para os controles (segmento, tecla, campo de texto, todos os botões). A barra de progresso tem 6 px de altura e cantos de 3 px. A pílula do rótulo é totalmente arredondada (999px); o indicador é um círculo de 4 px, o selo de 10 px e o ponto da linha de navegação de 8 px. O ladrilho é uma superelipse recortada por máscara SVG (`M50 0C90 0 100 10 100 50S90 100 50 100 0 90 0 50 10 0 50 0Z`); o contorno de foco segue `border-radius: 23%`, que aproxima a curva. O separador é um fio vertical de 1 px.

## Components

### Dock bar
Placa de vidro simulado, calma e baixa.
- **Superfície:** `rgb(var(--surface-rgb) / var(--surface-a))`, com a pilha da barra (Elevation).
- **Forma:** `--r-bar` do tamanho; padding `--pad`; itens separados por `--gap`.
- **Separador:** fio de 1 px em `--hair`, recuado 4 px em cima e embaixo, com margem lateral de `gap × 0,4`. Separa os widgets de Configurações e Sobre.

### Dock item / ladrilho
Cerâmica esmaltada, a assinatura do sistema.
- **Tamanho:** `--icon` (40/48/64 px), item é um `<button>` com `aria-label`.
- **Esmalte:** gradiente vertical da matiz do ladrilho, luminosidade +7% no topo e −5% na base, sob um brilho radial branco de 26% no terço superior (`radial-gradient(130% 70% at 50% -12%, …)`), mais as bordas do esmalte e a sombra do ladrilho.
- **Glifo:** SVG de traço em 24 px de viewBox, ocupando 54% do ladrilho, traço 1,75, pontas e junções arredondadas, branco. Um único conjunto de ícones de linha, desenhado no próprio projeto.
- **Hover:** ampliação em degraus (Layout), `transition` de `--t-magnify` com `--ease`.
- **Foco de teclado:** outline 2 px `--accent`, offset 3 px; também amplia e mostra a pílula. Navegação por setas, Home e End com roving tabindex.
- **Pressionado:** esmalte com `brightness(.84)`, glifo a 90%.
- **Ilustrativo:** saturação × 0,55.
- **Mini:** o mesmo ladrilho, estático, em 26 px na lista de itens e 18 px nos botões de adicionar; glifo a 60% com traço 2.

### Ícones vivos
O valor mora no esmalte; o ícone continua sendo um ladrilho, sem moldura nem card.
- **Relógio digital** (padrão): a hora em Tile Time, branca, centrada. `HH:mm` em 24 horas; em 12 horas, `h:mm` **sem AM/PM** no ícone (o AM/PM fica no painel).
- **Relógio analógico:** mostrador SVG a 76% do ladrilho, em 24 px de viewBox: quatro marcas de 0,75 de raio a 75% de opacidade, ponteiro das horas com traço 2,1 e comprimento 5, ponteiro dos minutos com traço 1,5 e comprimento 7,6, eixo de 1,25. Sem ponteiro de segundos. A escolha entre digital e analógico é um ajuste de cada relógio ("Ícone no dock").
- **Progresso:** anel SVG a 78% do ladrilho, raio 9, traço 2,2, trilho branco a 28% e arco branco do primeiro período, começando no topo. Percentual em Tile Pct no centro, arredondado para baixo. **No Pequeno, só o anel**, sem número. Sem período válido, o anel fica vazio e o centro mostra "–".
- **Atualização:** uma vez por minuto, alinhada à virada do minuto, com um único timer para o dock inteiro.

### Selo de aviso
Círculo de 10 px em `--danger`, no canto superior direito do ladrilho (−2 px), com anel de 2 px na cor da superfície. Aparece no ícone de Configurações quando o banco de dados foi recuperado ou está indisponível, porque o aviso mora em Configurações › Itens do dock. Decorativo para leitores de tela (`aria-hidden`); a informação está no aviso.

### Indicador de painel aberto
Ponto de 4 px em `--accent`, centrado sob o ícone, a 1 px da borda da barra (dentro do padding). Aparece com `aria-expanded="true"` por fade de 0,15 s. Na Fase 4 passa a ser o indicador de app em execução.

### Tooltip (pílula)
- **Estilo:** pílula 999px, `--tip-bg` / `--tip-ink`, Label Strong, padding 4px 10px, sombra da pílula.
- **Posição:** centrada no ladrilho, 10 px além dele (acima com o dock embaixo); reposicionada ao fim da transição de ampliação.
- **Regra:** não aparece para o item cujo painel está aberto.

### Painel
Folha opaca ancorada, com cara de flyout do Windows 11.
- **Forma:** 8 px, superfície opaca, pilha da barra com a sombra do painel, padding 16/16/12 px, grade com 14 px de intervalo.
- **Entrada:** `pop` de `--t-pop` com `--ease`, de `translateY(6px) scale(.98)` e opacidade 0; origem na borda voltada para o dock. Trocar de vista (Ajustes, Voltar, Itens do dock) re-renderiza a mesma folha, sem abrir outra janela.
- **Cabeçalho:** linha de 24 px de altura mínima com 6 px de intervalo. Na vista principal de um widget, título à esquerda e botão quieto "Ajustes" (ícone de controles + texto) à direita. Nas subtelas (Ajustes, Itens do dock), botão de ícone Voltar (seta para a esquerda, `aria-label="Voltar"`) antes do título. O foco volta ao controle de origem.
- **Configurações:** título; quatro linhas rótulo + controle segmentado (Tema: Sistema/Claro/Escuro; Posição: Embaixo/Em cima; Tamanho: Pequeno/Médio/Grande; Opacidade do dock: Translúcido/Sólido); linha de navegação "Itens do dock" com a contagem ("3 de 12", ou "Indisponível"); rodapé.
- **Configurações › Itens do dock:** subtela com Voltar. Lista na ordem do dock: ladrilho mini de 26 px, nome em Label Strong e subtítulo em Caption (fuso, períodos, linhas da nota), botões de ícone subir e descer (desligados nas pontas) e o de remover (Ícone de remover): Relógio e Progresso saem na hora; uma Nota com texto, guardado ou não salvo, pede a Confirmação inline na própria linha, ocupando a largura dela abaixo do nome. Depois de remover, o foco vai para o remover do item que ficou no lugar, do anterior no fim da lista, ou para o primeiro botão de adicionar. Linhas de 6 px de respiro com divisória `--hair`. A lista rola acima de 268 px de altura. Abaixo, "Adicionar" com a contagem à direita e botões secundários Relógio, Progresso e Nota, cada um com ladrilho mini de 18 px; com 12 widgets, a contagem vira "Limite de 12 widgets" e os botões ficam desligados. Sem widgets: "Nenhum widget no dock. Adicione um abaixo." Rodapé "A ordem é a do dock". Com o banco recuperado, o aviso vem antes da lista; com o banco indisponível, o aviso ocupa o lugar da lista e do adicionar.
- **Relógio** (300 px): hora em Clock Time, com segundos (Clock Seconds) e AM/PM (Clock AM/PM) em `--ink-2` quando ligados; data por extenso em Label `--ink-2`, primeira letra maiúscula; rodapé com fuso e UTC. **Ajustes do relógio:** Rótulo, Fuso horário (lista: "Do sistema", depois os fusos pelo deslocamento UTC atual, cada um como `UTC−3 · Sao Paulo · America/Sao_Paulo`), Formato (24 horas / 12 horas), Segundos no painel (Ocultar / Mostrar), Ícone no dock (Digital / Analógico), "Remover do dock".
- **Progresso do tempo** (320 px): lista de barras, 12 px entre itens; cada item numa grade de 78 px (nome) / barra / 42 px (percentual em Label Strong tabular, à direita), com "faltam …" em Caption `--ink-2` sob a barra. Período sem valor no momento fica em `--ink-2`, sem barra nem percentual, com "Fora do expediente" no lugar da barra. Rodapé com o início da semana. **Ajustes do progresso:** Rótulo; Períodos, na ordem do painel, como caixas de seleção com subir e descer (o último marcado não pode ser desmarcado; o expediente abre das/às com campos de hora de 84 px); dica "O primeiro período é o do anel no ícone."; Semana começa (Segunda / Domingo); "Remover do dock".
- **Nota** (340 px): campo de texto multilinha de 210 px de altura em Note Body, padding 8px 10px, sem redimensionar, rolagem fina. O estado de salvamento mora no rodapé: ponto de 6 px em `currentColor` a 55% + texto ("Salvo", "Salvando…" com o ponto piscando, ou o erro "Passou de 64 KB e não foi salvo. Encurte o texto." em `--danger`, com o ponto pleno e quebra de linha permitida). **Ajustes da nota:** Rótulo; "Remover do dock", que vira confirmação inline quando a nota tem texto (nota vazia sai sem perguntar).
- **Sobre:** ladrilho de 56 px + nome (Title About) e descrição; lista chave/valor (Versão, Licença, Código com o endereço do repositório em texto selecionável em `--ink`; vira link em `--accent-ink` na Fase 3, com o `opener`); rodapé.
- **Rodapé:** Caption em `--ink-2`, fio superior `--hair`, 10 px de respiro, texto à esquerda (corta com reticências, salvo o erro da Nota) e dica de tecla à direita.
- **Fechamento:** Esc (devolve o foco ao item), clique fora ou novo clique no item.

### Controle segmentado
- **Trilho:** `--track`, raio 6 px, padding e gap de 2 px.
- **Segmento:** transparente, Label, raio 4 px, padding 6 px, largura igual entre os segmentos. Hover: `--track`.
- **Selecionado:** `--accent-soft`, `--accent-ink`, peso 600, anel interno de 1 px no cobalto a 40%. `aria-pressed` comunica o estado.
- **Foco:** outline 2 px `--accent`, offset 1 px.

### Buttons
Todos em Label (12,5 px), raio 4 px, sem borda, com ícones de linha de 16 px e traço 1,75 do próprio projeto. Foco: outline 2 px `--accent`, offset 1 px.
- **Quieto:** transparente, `--ink-2`, padding 4px 8px, ícone + texto com 5 px de intervalo ("Ajustes"). Hover: `--track` e `--ink`.
- **Ícone:** 26 × 26 px, transparente, `--ink-2` (Voltar, subir, descer). Hover: `--track` e `--ink`. Desligado: opacidade 0,32, sem hover.
- **Ícone de remover:** o botão de ícone com a lixeira, em `--ink-2` em repouso; no hover, `--danger-soft` e `--danger` (remove, The Danger Only Rule). Nome acessível "Remover {nome}". Só em Itens do dock.
- **Secundário:** `--track`, `--ink`, padding 6px 10px ("Entendi", "Cancelar", adicionar). Hover: tinta a 11%. Desligado: opacidade 0,4.
- **Perigo:** `--danger`, `--danger-ink`, peso 600, padding 6px 12px ("Remover nota"); só dentro da confirmação. Hover: `brightness(1.08)`.
- **Remover:** transparente, texto e ícone de lixeira em `--danger`, padding 6px 8px, alinhado à esquerda ("Remover do dock"); fica no fim dos Ajustes de todo widget. Hover: `--danger-soft`.

### Linha de navegação
Botão de largura total que leva a uma subtela: `--track`, raio 6 px, padding 8px 8px 8px 10px. Rótulo em Label Strong à esquerda; valor em `--ink-2` tabular à direita, com o ponto de aviso de 8 px em `--danger` quando há aviso; seta para a direita em `--ink-2`. Hover: tinta a 9%.

### Inputs / Fields
O campo de texto do Windows 11.
- **Estilo:** `--field`, borda de 1 px em `--hair` com o fio inferior em `--field-line`, raio 4 px, padding 6px 8px, Label em `--ink`; placeholder em `--ink-2` cheio (a 80% ficava em ~3,98:1 sobre `--field`, abaixo de 4,5:1). Rótulo acima em Label `--ink-2`, 6 px de intervalo. Vale para texto, lista (select), hora e a Nota.
- **Foco:** sem outline; o fio inferior vira `--accent` com 1 px interno a mais (2 px no total), cursor em `--accent`.
- **Recusado** (`aria-invalid`): o fio inferior vira `--ink` com 1 px interno a mais; a mensagem vai para o rodapé do painel, em `--ink`. Nunca `--danger` (The Danger Only Rule): recusa de validação não é erro de salvamento.
- **Caixa de seleção:** nativa, 15 px, `accent-color: var(--accent)`.
- **Controles nativos:** `color-scheme` acompanha o tema em `tokens.css` (`light` no bloco claro, `dark` nos dois blocos escuros, o de Sistema e o de Escuro forçado). Assim o campo de hora, a caixa de seleção e as barras de rolagem saem no visual nativo do mesmo tema (teste manual de 2026-10-16). A lista aberta do `<select>` não: é uma janela própria do WebView2 que pinta uma superfície clara em todo tema e ignora o `color-scheme`; só a cor do texto das `<option>` vale. Por isso as `<option>` recebem a tinta clara fixa `--popup-ink` (igual à `--ink` do tema claro, definida só em `:root`). Ver o ADR 0007.

### Aviso inline (notice)
Bloco `--track`, raio 6 px, padding 10px 12px, grade de ícone de 16 px + texto. Ícone de alerta em `--danger`; texto em Label `--ink-2` com a frase principal em Label Strong `--ink`. Ação opcional como botão secundário sob o texto ("Entendi"). `role="status"`. Usado para banco recuperado (com o nome do arquivo guardado) e banco indisponível.

### Confirmação inline
Bloco `--danger-soft`, raio 6 px, padding 12px, 10 px entre texto e ações. Frase principal em Label Strong ("Remover a nota apaga o texto.") seguida da consequência ("Não dá para desfazer."). Ações à direita, 6 px de intervalo: secundário "Cancelar" e perigo "Remover nota". `role="alert"`. Substitui o botão "Remover do dock" no lugar dele; nunca é modal.

### Dica de tecla
`<kbd>` em Kbd, padding 3px 6px, raio 4 px, fundo `--kbd`, texto `--ink`.

### Named Rules
**The Same Sheet Rule.** Detalhe, Ajustes e subtelas de um painel são vistas da mesma folha, trocadas por "Ajustes" e Voltar. Confirmação e aviso aparecem dentro da folha, no lugar do controle que os disparou. Nada de modal, diálogo ou segunda janela.

## Do's and Don'ts

### Do:
- **Do** guardar o cobalto (`--accent`) para foco, seleção e o ponto de painel aberto.
- **Do** guardar o perigo (`--danger`) para a confirmação de remoção, o erro de salvamento e o selo de aviso.
- **Do** manter o painel opaco (`rgb(var(--surface-rgb))`) nas duas opacidades do dock.
- **Do** escalar barra, ícone, padding, gap e raio juntos pelos presets s/m/l (40/48/64, 6/7/9, 6/7/9, 18/20/24).
- **Do** usar 8 px nos cantos do painel, como as janelas e flyouts do Windows 11.
- **Do** criar ladrilhos novos como classes estáticas `.t-*` com `--h`, `--sat`, `--l` no registro dessaturado da paleta, com glifo de traço branco 1,75 a 54%.
- **Do** escrever o valor vivo em branco no próprio esmalte, em Display 600 tabular, e atualizar uma vez por minuto.
- **Do** mostrar só o anel do Progresso no tamanho Pequeno.
- **Do** confirmar e avisar dentro da folha, no lugar do controle que disparou.
- **Do** inverter a luz de borda e a origem da ampliação quando o dock vai para a borda de cima.
- **Do** congelar a ampliação enquanto um painel está aberto e desligá-la com "reduzir movimento".
- **Do** declarar tokens em CSS estático e aplicar valores por item via CSSOM (`setProperty`).

### Don't:
- **Don't** usar desfoque real do fundo (`backdrop-filter` ou backdrops do DWM); o vidro é simulado (ADR 0006).
- **Don't** usar cobalto, perigo ou qualquer cor de marca nos ladrilhos.
- **Don't** usar o perigo como decoração, destaque ou estado de foco.
- **Don't** usar ícones, temas ou arquivos de terceiros (WhiteSur, Plank, Latte e afins são só referência).
- **Don't** fazer a barra escura genérica de launcher nem o arco-íris saturado de ícones da Apple.
- **Don't** fazer grade de cards de widget nem painel lateral de widgets; o widget é um ladrilho do dock com o próprio flyout.
- **Don't** abrir modal ou diálogo para confirmar ou avisar.
- **Don't** pôr AM/PM no ícone do Relógio.
- **Don't** carregar fontes web; a CSP estrita não permite e a fonte é a do sistema.
- **Don't** injetar atributos `style` ou `<style>` em runtime.
- **Don't** sobrepor ícones ao ampliar: a ampliação empurra os vizinhos.
