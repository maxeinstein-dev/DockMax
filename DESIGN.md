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
  glifo: "#ffffff"
  salvia: "hsl(150 22% 44%)"
  ocre: "hsl(38 48% 40%)"
  petroleo: "hsl(192 26% 40%)"
  argila: "hsl(16 40% 44%)"
  ameixa: "hsl(318 20% 43%)"
  tinta: "hsl(222 14% 34%)"
  ardosia: "hsl(214 22% 46%)"
typography:
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
  segment: "4px"
  kbd: "4px"
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

**Key Characteristics:**
- Placa de vidro simulado com cantos grandes, proporcionais ao tamanho (18/20/24 px).
- Ladrilhos em superelipse com esmalte de dois tons da mesma matiz e glifo de traço branco 1,75.
- Destaque único azul-cobalto, reservado a foco, seleção e ponto de painel aberto.
- Ampliação em degraus que empurra os vizinhos, com rótulo em pílula acima do ícone.
- Painel sempre opaco, com cantos de 8 px como as janelas e flyouts do Windows 11.
- Dois temas por `[data-mode]`, duas opacidades do dock, três tamanhos, duas bordas: tudo troca de valor, nunca de componente.

## Colors

Neutros frios quase sem matiz, um azul-cobalto contido e uma paleta de esmaltes terrosos e frios, todos dessaturados para não cansar ao longo do dia.

### Primary
- **Azul-cobalto** (`cobalto-light` / `cobalto-dark`, token `--accent`): contorno de foco do ladrilho e dos segmentos (2 px), anel interno do segmento selecionado (a 40%) e ponto de painel aberto. No escuro, sobe para um cobalto claro para manter contraste sobre o grafite.
- **Tinta de cobalto** (`cobalto-ink-light` / `cobalto-ink-dark`, token `--accent-ink`): texto do segmento selecionado e links do painel Sobre.
- **Véu de cobalto** (`cobalto-soft-light` / `cobalto-soft-dark`, token `--accent-soft`): fundo do segmento selecionado.

### Tertiary (esmaltes dos ladrilhos)
Classes estáticas `.t-*`, definidas por matiz, saturação e luminosidade (`--h`, `--sat`, `--l`), para que o esmalte seja calculado em CSS sem estilo inline.
- **Tinta** (`tinta`): Configurações. O tom mais sóbrio, para a ferramenta.
- **Ardósia** (`ardosia`): Sobre.
- **Sálvia** (`salvia`), **Ocre** (`ocre`), **Petróleo** (`petroleo`), **Argila** (`argila`), **Ameixa** (`ameixa`): reservados aos itens das fases 2 e 3 (Arquivos, Notas, Relógio, Calendário, Música). Enquanto forem ilustrativos, entram com a saturação multiplicada por 0,55 (`--sat-k`).
- **Glifo** (`glifo`): o traço branco sobre todo ladrilho.

### Neutral
- **Porcelana fosca** (`porcelana-surface`, token `--surface-rgb: 247 247 249`): superfície da barra e do painel no tema claro.
- **Grafite fosco** (`grafite-surface`, token `--surface-rgb: 38 39 44`): superfície da barra e do painel no tema escuro.
- **Tinta** (`ink-*`, `--ink`): texto principal do painel e cor herdada do dock.
- **Tinta secundária** (`ink-2-*`, `--ink-2`): rótulos das linhas do painel, rodapé, chaves da lista do Sobre.
- **Fio** (`hair-*`, `--hair`): anel externo de 1 px da barra, do painel e da pílula; separador; linha do rodapé.
- **Luz de borda** (`hi-*`, `--hi`): o 1 px interno que pega luz na barra e no painel.
- **Pílula** (`tip-bg-*`, `tip-ink-*`): fundo quase opaco (97%) e texto do rótulo.
- **Trilho** (`track-*`, `--track`): fundo do controle segmentado e hover do segmento.
- **Tecla** (`kbd-*`, `--kbd`): fundo da dica de teclado.

### Named Rules
**The One Cobalt Rule.** O cobalto existe só para foco, seleção e painel aberto. Nunca em ladrilho, superfície, texto corrido ou decoração. Se um elemento azul não responde a uma dessas três perguntas, está errado.

**The Own Glaze Rule.** Todo ladrilho sai da paleta própria, em classe estática. Nenhum ícone, cor de marca ou arquivo de tema de terceiros; um item novo ganha uma matiz nova no mesmo registro dessaturado (saturação 14 a 48%, luminosidade 34 a 46%).

**The Swappable Identity Rule.** Toda cor é um token por tema em `[data-mode="light"]` / `[data-mode="dark"]`. O destaque é um token só, para que a escolha de destaque pelo usuário entre depois sem retrabalho.

## Typography

**Display Font:** Segoe UI Variable Display (com Segoe UI Variable, Segoe UI, system-ui)
**Body Font:** Segoe UI Variable Text (com Segoe UI Variable, Segoe UI, system-ui)

**Character:** A fonte do próprio Windows 11, em corpo pequeno e peso firme: o dock deve parecer parte do shell, não um site. Não há fontes web; a CSP estrita do app não as carrega e o mock também não.

### Hierarchy
- **Title** (600, 15px, 1,3, Display): título do painel ("Configurações").
- **Title About** (600, 17px, 1,3, Display): nome "DockMax" ao lado do ladrilho de 56 px no painel Sobre; única exceção ao título de 15px.
- **Label** (400, 12,5px): rótulos das linhas do painel, segmentos, linhas do Sobre.
- **Label Strong** (600, 12,5px): texto da pílula e segmento selecionado.
- **Caption** (400, 12px): rodapé do painel ("Salvo automaticamente", "Projeto independente").
- **Kbd** (600, 11px, 1): dica de tecla (`Esc`).

### Named Rules
**The Shell Font Rule.** Só Segoe UI Variable, em pesos 400 e 600. Display apenas em títulos de painel; o resto é Text.

## Layout

O dock é centralizado na borda inferior (ou superior) do monitor, 8 px acima da área útil (`edge-offset`). A barra é uma linha flexível de itens alinhada à borda da tela, com altura `icon + 2 × pad` e o mesmo valor para `pad` e `gap` em cada tamanho:

| Tamanho | `--icon` | `--pad` | `--gap` | `--r-bar` |
|---|---|---|---|---|
| Pequeno (`s`) | 40px | 6px | 6px | 18px |
| Médio (`m`) | 48px | 7px | 7px | 20px |
| Grande (`l`) | 64px | 9px | 9px | 24px |

**Ampliação (B1, em degraus).** O item sob o mouse (ou com foco de teclado) escala para 1,4 e sobe 6 px; os vizinhos imediatos escalam para 1,15. A largura do item cresce junto (`width: icon × s`), então a ampliação **empurra os vizinhos** e a barra cresce na horizontal, em vez de sobrepor. O ladrilho cresce a partir da borda da tela (`transform-origin` na base, ou no topo com o dock em cima). Congela enquanto há painel aberto e é desligada com "reduzir movimento".

**Painel.** Ancorado ao item: centrado nele, 8 px além da barra, limitado a 8 px das bordas da tela. Largura 320 px (Configurações) ou 300 px (Sobre); grade de uma coluna com 14 px entre linhas e padding 16/16/12 px.

**Restrições para os PRs 2 e 3.**
- A janela do dock precisa de folga lateral além da de cima, porque a barra alarga ao ampliar.
- No tamanho Grande, o item ampliado chega a ~105 px acima da base (64 × 1,4 + 6 + 9), contra os 104 px da janela atual (barra de 64 + folga de 40). O PR 2 precisa aumentar a folga conforme o tamanho ou limitar a ampliação no Grande.
- Tokens em CSS estático. Escalas por item via CSSOM (`style.setProperty("--s", …)`) são compatíveis com a CSP; atributos `style` injetados no HTML não são.

## Elevation & Depth

Híbrido de vidro simulado e sombra ambiente. Não existe desfoque do fundo: a profundidade vem de três camadas empilhadas na mesma `box-shadow` (luz de borda interna, anel de fio de 1 px, sombra macia deslocada para fora da tela) e da opacidade da superfície. O dock tem duas opacidades (`--surface-a`: 0,8 Translúcido, 1 Sólido); o painel é sempre opaco.

### Shadow Vocabulary
- **Sombra da superfície, claro** (`--shadow: 0 1px 2px rgb(16 20 32 / .10), 0 14px 32px -12px rgb(16 20 32 / .42)`): barra.
- **Sombra da superfície, escuro** (`--shadow: 0 1px 2px rgb(0 0 0 / .32), 0 16px 36px -12px rgb(0 0 0 / .7)`): barra.
- **Sombra do painel, claro** (`--shadow-panel: 0 1px 2px rgb(16 20 32 / .10), 0 2px 8px -2px rgb(16 20 32 / .24)`) e **escuro** (`--shadow-panel: 0 1px 2px rgb(0 0 0 / .32), 0 2px 8px -2px rgb(0 0 0 / .5)`): a mesma sombra em versão curta, que cabe nos 8 px de margem transparente da janela do painel (a janela bloqueia cliques e não pode cobrir a barra do dock, ADR 0006).
- **Pilha da barra** (`box-shadow: inset 0 1px 0 var(--hi), 0 0 0 1px var(--hair), var(--shadow)`): com o dock em cima, a luz interna passa para a borda de baixo (`inset 0 -1px 0`). O painel usa a mesma pilha com a sombra do painel no lugar de `--shadow`.
- **Sombra da pílula** (`box-shadow: 0 0 0 1px var(--hair), 0 6px 16px -6px rgb(0 0 0 / .35)`).
- **Sombra do ladrilho** (`filter: drop-shadow(0 1px 1px rgb(0 0 0 / .18)) drop-shadow(0 5px 6px rgb(0 0 0 / .16))`): em `filter` no invólucro sem máscara, para seguir a silhueta da superelipse.
- **Bordas do esmalte** (`box-shadow: inset 0 1px 0 rgb(255 255 255 / .32), inset 0 -1px 0 rgb(0 0 0 / .16)`).

### Named Rules
**The Opaque Panel Rule.** O painel é sempre opaco (`rgb(var(--surface-rgb))`), qualquer que seja a opacidade do dock. Sem desfoque, texto translúcido sobre o Chrome de trás não se lê.

**The Light Edge Rule.** A luz de borda de 1 px fica sempre no lado oposto à borda da tela: em cima com o dock embaixo, embaixo com o dock em cima.

## Shapes

Três famílias de canto. A barra tem cantos grandes que crescem com o tamanho (18/20/24 px), à maneira do Big Sur. O painel usa 8 px, os cantos das janelas e flyouts do Windows 11; dentro dele, trilho de 6 px, segmento e tecla de 4 px. A pílula do rótulo é totalmente arredondada (999px) e o indicador é um círculo de 4 px. O ladrilho é uma superelipse recortada por máscara SVG (`M50 0C90 0 100 10 100 50S90 100 50 100 0 90 0 50 10 0 50 0Z`); o contorno de foco segue `border-radius: 23%`, que aproxima a curva. O separador é um fio vertical de 1 px.

## Components

### Dock bar
Placa de vidro simulado, calma e baixa.
- **Superfície:** `rgb(var(--surface-rgb) / var(--surface-a))`, com a pilha da barra (Elevation).
- **Forma:** `--r-bar` do tamanho; padding `--pad`; itens separados por `--gap`.
- **Separador:** fio de 1 px em `--hair`, recuado 4 px em cima e embaixo, com margem lateral de `gap × 0,4`. Separa os itens das próximas fases de Configurações e Sobre.

### Dock item / ladrilho
Cerâmica esmaltada, a assinatura do sistema.
- **Tamanho:** `--icon` (40/48/64 px), item é um `<button>` com `aria-label`.
- **Esmalte:** gradiente vertical da matiz do ladrilho, luminosidade +7% no topo e −5% na base, sob um brilho radial branco de 26% no terço superior (`radial-gradient(130% 70% at 50% -12%, …)`), mais as bordas do esmalte e a sombra do ladrilho.
- **Glifo:** SVG de traço em 24 px de viewBox, ocupando 54% do ladrilho, traço 1,75, pontas e junções arredondadas, branco. Um único conjunto de ícones de linha, desenhado no próprio projeto.
- **Hover:** ampliação em degraus (Layout), `transition` de `--t-magnify` com `--ease`.
- **Foco de teclado:** outline 2 px `--accent`, offset 3 px; também amplia e mostra a pílula. Navegação por setas, Home e End com roving tabindex.
- **Pressionado:** esmalte com `brightness(.84)`, glifo a 90%.
- **Ilustrativo:** saturação × 0,55.

### Indicador de painel aberto
Ponto de 4 px em `--accent`, centrado sob o ícone, a 1 px da borda da barra (dentro do padding). Aparece com `aria-expanded="true"` por fade de 0,15 s. Na Fase 4 passa a ser o indicador de app em execução.

### Tooltip (pílula)
- **Estilo:** pílula 999px, `--tip-bg` / `--tip-ink`, Label Strong, padding 4px 10px, sombra da pílula.
- **Posição:** centrada no ladrilho, 10 px além dele (acima com o dock embaixo); reposicionada ao fim da transição de ampliação.
- **Regra:** não aparece para o item cujo painel está aberto.

### Painel
Folha opaca ancorada, com cara de flyout do Windows 11.
- **Forma:** 8 px, superfície opaca, pilha da barra com a sombra do painel, padding 16/16/12 px, grade com 14 px de intervalo.
- **Entrada:** `pop` de `--t-pop` com `--ease`, de `translateY(6px) scale(.98)` e opacidade 0; origem na borda voltada para o dock.
- **Configurações:** título; quatro linhas rótulo + controle segmentado (Tema: Sistema/Claro/Escuro; Posição: Embaixo/Em cima; Tamanho: Pequeno/Médio/Grande; Opacidade do dock: Translúcido/Sólido); rodapé.
- **Sobre:** ladrilho de 56 px + nome (Title About) e descrição; lista chave/valor (Versão, Licença, Código com link em `--accent-ink`); rodapé.
- **Rodapé:** Caption em `--ink-2`, fio superior `--hair`, 10 px de respiro, texto à esquerda e dica de tecla à direita.
- **Fechamento:** Esc (devolve o foco ao item), clique fora ou novo clique no item.

### Controle segmentado
- **Trilho:** `--track`, raio 6 px, padding e gap de 2 px.
- **Segmento:** transparente, Label, raio 4 px, padding 6 px, largura igual entre os segmentos. Hover: `--track`.
- **Selecionado:** `--accent-soft`, `--accent-ink`, peso 600, anel interno de 1 px no cobalto a 40%. `aria-pressed` comunica o estado.
- **Foco:** outline 2 px `--accent`, offset 1 px.

### Dica de tecla
`<kbd>` em Kbd, padding 3px 6px, raio 4 px, fundo `--kbd`, texto `--ink`.

## Do's and Don'ts

### Do:
- **Do** guardar o cobalto (`--accent`) para foco, seleção e o ponto de painel aberto.
- **Do** manter o painel opaco (`rgb(var(--surface-rgb))`) nas duas opacidades do dock.
- **Do** escalar barra, ícone, padding, gap e raio juntos pelos presets s/m/l (40/48/64, 6/7/9, 6/7/9, 18/20/24).
- **Do** usar 8 px nos cantos do painel, como as janelas e flyouts do Windows 11.
- **Do** criar ladrilhos novos como classes estáticas `.t-*` com `--h`, `--sat`, `--l` no registro dessaturado da paleta, com glifo de traço branco 1,75 a 54%.
- **Do** inverter a luz de borda e a origem da ampliação quando o dock vai para a borda de cima.
- **Do** congelar a ampliação enquanto um painel está aberto e desligá-la com "reduzir movimento".
- **Do** declarar tokens em CSS estático e aplicar valores por item via CSSOM (`setProperty`).

### Don't:
- **Don't** usar desfoque real do fundo (`backdrop-filter` ou backdrops do DWM); o vidro é simulado (ADR 0006).
- **Don't** usar cobalto ou qualquer cor de marca nos ladrilhos.
- **Don't** usar ícones, temas ou arquivos de terceiros (WhiteSur, Plank, Latte e afins são só referência).
- **Don't** fazer a barra escura genérica de launcher nem o arco-íris saturado de ícones da Apple.
- **Don't** carregar fontes web; a CSP estrita não permite e a fonte é a do sistema.
- **Don't** injetar atributos `style` ou `<style>` em runtime.
- **Don't** sobrepor ícones ao ampliar: a ampliação empurra os vizinhos.
