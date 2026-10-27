# Spec 0014 — Rebranding de Configurações e dos menus

**Status:** R1 e R2 feitos; R3 (menus próprios) implementado; R4 a fazer.
**Origem:** pedido do autor (2026-10-27): Configurações, os menus do dock e da bandeja e as outras telas de apoio estão "muito simples e sem vida". Referência de acabamento: o PrimoDock ([interface](../reference/primodock/interface.md), "Janela de configurações" e "Menus"), com o tema WhiteSur GTK e o dock do ZorinOS como inspiração. Exemplo dado pelo autor: em Aparência, mostrar os temas aplicados, e não só os nomes.

## 1. Decisões do autor (2026-10-27)

| Pergunta | Decisão |
|---|---|
| Por onde começar | **Configurações primeiro** (estrutura e temas com miniatura); os menus num PR seguinte |
| Miniatura dos temas | **Dock em miniatura ao vivo**, desenhado em CSS, com os apps do usuário e as mudanças que ele fez no tema |
| Menus | **Menu próprio desenhado** (janela nossa, como o lançador), não o menu do Windows com ícones |
| Cor de destaque | **A do DockMax** (`--accent`, o cobalto), não o coral do PrimoDock |

## 2. O que existe hoje

- **Configurações** (spec 0011 H1): uma janela com a barra lateral só de texto (Geral, Espaços de trabalho, Aparência do dock, Apps do dock, Itens do dock, Sobre) e, ao lado, as vistas que eram do painel, numa coluna de até 640 px. Cada escolha é uma linha de rótulo com um controle segmentado, uma chave ou uma lista; o tema do dock é uma **lista de texto** (`SelectField`).
- **Menu do dock** (botão direito no espaço vazio): o menu do Windows (`TrackPopupMenu`, `native_dock/mod.rs`), sem ícones.
- **Menu de um programa** (botão direito num ícone): um painel do WebView2 (`AppMenuPanel`), com a Jump List.
- **Menu do ícone do DockMax na bandeja:** o menu do Tauri (`tray.rs`), que é o do Windows.

## 3. Linguagem visual

Vale para todos os recortes. O que muda entra no `DESIGN.md` junto com o recorte que o usa.

- **Placa colorida com ícone de linha:** cada seção de Configurações (e, no R3, cada grupo de menu que tiver) tem uma placa de cantos arredondados no esmalte próprio do DockMax (The Own Glaze Rule: matiz própria, saturação de 14 a 48 %, luminosidade de 34 a 46 %) com um ícone de linha branco por cima, como os ladrilhos dos widgets. Nada de ícones ou cores de terceiros.
- **Cabeçalho de seção:** placa grande, título em Display e uma frase de apoio em `--ink-2`.
- **Cartões de grupo:** um título pequeno em `--ink-2` e, abaixo, um cartão arredondado um pouco mais claro que o fundo, com as linhas separadas por fios `--hair`. Cada linha: nome em Label Strong, descrição em `--ink-2` embaixo, controle à direita.
- **Escolhas visuais como cartões com miniatura**, em grade: o escolhido com o anel no cobalto e o nome em negrito (seleção, The One Cobalt Rule).
- **Destaque:** o `--accent` de sempre, só para foco, seleção e o que está aberto. A aba ativa ganha o fundo `--track` e o traço cobalto à esquerda.

## 4. Recortes

| Recorte | Entrega |
|---|---|
| **R1. Estrutura de Configurações e temas com miniatura** | Barra lateral com placa e ícone em cada seção; cabeçalho de seção; grupos em cartões nas seções Geral e Aparência; **o tema do dock como uma grade de cartões, cada um com o dock em miniatura naquele tema**, com os apps fixados do espaço em uso e as mudanças do usuário (borda, cantos, reflexos). |
| **R2. As outras escolhas visuais** | Cartões com miniatura para posição, efeito ao passar o mouse (a miniatura do escolhido anima o efeito), indicador, formato dos ícones, animação de abertura e a barra de tarefas do Windows; o valor dos controles deslizantes em algarismos tabulares; as seções Espaços, Apps, Itens e Sobre na mesma linguagem. |
| **R3. Menus próprios** | Uma janela de menu nativa (Direct2D, como o lançador): cartão arredondado com sombra, ícone de linha em cada item, o item sob o ponteiro preenchido em `--accent`, marcas e atalhos à direita, títulos de grupo, as bolinhas dos espaços. Substitui o menu do dock e o do ícone da bandeja; o menu de um programa (`AppMenuPanel`) ganha a mesma aparência. Teclado (setas, Enter, Esc, letra de acesso) e UI Automation como no menu do Windows. |
| **R4. Outras telas de apoio** | O painel da bandeja, os widgets abertos e o lançador conferidos contra a linguagem nova. |

Cada recorte é um PR, com a verificação no app real nos dois temas e a medição de memória (Configurações abre o WebView2 só enquanto está aberta; o menu do R3 não pode fazer o dock nativo crescer de forma visível).

## 5. R1 em detalhe

### 5.1 Barra lateral

- Largura de 220 px. No topo, o ladrilho do DockMax e "DockMax / Configurações".
- Cada seção é um botão com uma **placa de 24 px** (raio 6 px) no esmalte da seção, com o ícone de linha branco de 14 px, e o nome. A ativa tem o fundo `--track`, peso 600 e o traço cobalto de 2 px à esquerda (`aria-current="page"`).

| Seção | Esmalte | Ícone |
|---|---|---|
| Geral | Tinta (222 14 % 34 %, `.t-settings`) | engrenagem |
| Espaços de trabalho | Petróleo (192 26 % 40 %) | quatro quadrados |
| Aparência do dock | Ameixa (318 20 % 43 %) | pincel |
| Apps do dock | Sálvia (150 22 % 44 %) | grade de apps |
| Itens do dock | Ocre (38 48 % 40 %) | widgets |
| Sobre | Ardósia (214 22 % 46 %, `.t-about`) | "i" |

### 5.2 Cabeçalho e grupos

- Toda seção começa com o cabeçalho: placa de 40 px, título em Display e uma frase de apoio (Geral: "Como o DockMax se encaixa no Windows."; Aparência: "Deixe o dock com a sua cara."; as outras no texto do R1).
- **Geral** em grupos: **Dock** (tema claro/escuro, posição, tamanho, opacidade), **Barra de tarefas do Windows**, **Comportamento** (todos os monitores, ocultar automaticamente e os atrasos, mostrar apps abertos, clique do meio), **Lançador** (chave, atalho, palavras-chave da web) e **Inicialização** (abrir com o Windows).
- **Aparência** em grupos: **Tema** (a grade do §5.3), **Personalizar este tema** (borda, cantos, reflexos, restaurar), **Ao passar o mouse**, **Indicador**, **Ícones** (formato e pacote) e **Animação de abertura**. No R1 esses grupos continuam com os controles de hoje; os cartões com miniatura deles são do R2.

### 5.3 Os temas com miniatura

- Uma grade de 3 colunas com os nove temas, na ordem de hoje. Cada cartão é um botão: a miniatura em cima, o nome embaixo. Um grupo de botões com `aria-pressed` e as setas para andar entre eles, como o `SwatchGroup`.
- **A miniatura** é um papel de parede neutro (um degradê escuro fixo, igual em todos os cartões, para os temas claros e escuros aparecerem) com um **dock em miniatura** no pé: a barra no tema, com a borda, os cantos e os reflexos que o usuário escolheu para aquele tema, e até cinco ícones dos **programas fixados no espaço em uso** (os ícones de verdade que o dock já lê); sem programas, ladrilhos dos widgets. O indicador aparece embaixo do primeiro ícone.
- **As cores** de cada tema vêm de uma tabela no TypeScript, espelho de `native_dock/look.rs` (preenchimento, separador, brilho, contorno do Neon, prateleira da Prateleira Aqua). Um teste compara a tabela com os valores do Rust, para as duas não se separarem.
- Escolher um cartão aplica o tema na hora, como a lista fazia. A lista some.
- Sem animação nas miniaturas no R1, e nada de WebView2 a mais: a miniatura é HTML e CSS dentro da página de Configurações.

### 5.4 Fora do R1

A busca nos ajustes, a prévia do dock sobre o papel de parede de verdade, a aparência dos menus (R3) e as miniaturas das outras escolhas (R2).

## 5c. Como o R3 ficou

- **Uma janela de menu nativa** (`windows/menu/`, classe `DockMaxMenu`), desenhada com Direct2D como o lançador e com as mesmas cores (o azul de seleção do lançador; as do Windows no contraste alto): cartão de cantos arredondados (o Windows 11 põe a borda e a sombra), um ícone de linha (da fonte de ícones do Windows) ou a bolinha do espaço em cada item, o item sob o ponteiro ou o teclado preenchido no azul, o atalho em cinza à direita, a marca (✓) no azul, títulos de grupo e separadores. A largura segue o texto mais largo (de 240 a 420 px).
- **Funciona como o menu do Windows** (`show` espera a escolha, como o `TrackPopupMenu`): abre acima do clique com o dock embaixo e abaixo com o dock em cima, sempre dentro da tela; setas, Home, End, Enter, Espaço e a primeira letra (sem acento) escolhem; Esc, Alt, F10 e um clique fora fecham.
- **UI Automation:** um menu com itens (nome, atalho, posição, ligado ou desligado em "Ocultar automaticamente"), separadores e o título; o item ativo recebe o foco, e "Invocar" escolhe. O leitor de tela ouve o menu abrir e fechar.
- **Onde:** o menu do dock (agora com o título "Espaços de trabalho" sobre os espaços, cada um com a sua cor) e o do ícone da bandeja, que deixou o menu do Tauri e é o mesmo do dock sem os espaços. A detecção de menu na tela (que segura o dock levantado) conta o menu novo.
- **O menu de um programa** (o painel com a Jump List) ganhou a mesma aparência, em CSS: itens mais altos de cantos arredondados, o item sob o ponteiro ou o foco preenchido no azul, ícones de linha nas ações do dock (abrir, mostrar no Explorador, fechar, remover).
- Os menus dos ícones da bandeja de outros programas continuam os deles.

## 6. Verificação

- Testes da barra lateral (seções, a ativa, o teclado), do cabeçalho e da grade de temas (escolher, as setas, o tema com as mudanças do usuário, sem programas fixados); o teste da tabela de cores contra o Rust.
- No app real (release), nos dois temas: capturas de Configurações (Geral e Aparência) e a troca de tema pelo cartão mudando o dock.
- Medir a memória com Configurações aberta, antes e depois.

**Verificado (R1, 2026-10-27),** com a cópia de teste (release, dados temporários), sem mexer no ponteiro:

- Configurações abriu pelo lançador; capturas de Geral e Aparência nos temas claro e escuro: a barra lateral com as placas, o cabeçalho de cada seção, os grupos em cartões e os nove temas com o dock em miniatura (sem programas fixados nos dados de teste, as miniaturas mostram os ladrilhos dos widgets);
- o cartão Neon salvou `neon` e mudou o dock; o Nativo voltou; o tema Escuro foi salvo e Configurações o seguiu;
- Esc fechou Configurações; o "Sair" saiu sem queda no log, sem processo sobrando e com a barra do Windows igual.
- **Memória privada** (processo e WebView2): 212 MB com o dock aberto e 243 MB com Configurações aberta na Aparência. Não houve medição da main na mesma rodada, então a diferença do R1 em si não foi medida; Configurações só existe enquanto está aberta.

**Não verificado (R1):** as miniaturas com programas fixados de verdade, com o mouse, e o leitor de tela na grade de temas.
