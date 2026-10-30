# Spec 0020 — Temas

Extensão da fase G ([spec 0019](0019-fase-g-extras.md)): **temas** que vestem o dock de uma vez, como os temas sazonais do PrimoDock, começando por temas gamer.

**Status:** em andamento: passo 1 (galeria de Temas), passo 2 (launchers) e passo 3 (skins de barra, com Retrô, Clássico 98 e Fósforo) passo 4 (Tático) e passo 5 (Medieval 8-bit).

## 1. Decisões do autor (2026-10-30)

Tomadas numa sessão de perguntas (Q1–Q22), todas na recomendação:

| # | Decisão |
|---|---|
| Q1 | Temas **inspirados**, com nome e desenho próprios: nada de nome, logo, fonte ou arte de jogo (CS2, Tibia, Monster Hunter, Call of Duty são marcas de terceiros). |
| Q2, Q6 | Um **Tema** é um atalho: aplica de uma vez o estilo da barra, o pacote de ícones e a cor do indicador. Nada novo é guardado: o tema em uso é o que tem as três escolhas do dock, e nenhum quando o usuário muda uma delas ("Personalizado"). |
| Q3 | Primeira leva gamer: **Tático** (FPS), **Medieval 8-bit** (MMORPG) e **Caçador** (fantasia de caça). Cyberpunk, arcade e sci-fi ficam para depois. |
| Q4, Q12 | Os pacotes cobrem os apps de sempre e os launchers de jogador (Epic, Battle.net, Riot, EA, Ubisoft, GOG, Faceit, NVIDIA, AMD, Overwolf, Twitch, Medal), que entram também nos pacotes vetoriais. Os executáveis dos jogos ficam com o ícone do sistema ou a reserva. |
| Q5 | Nomes: Geral › Tema vira **Modo de cor**; Aparência › "Tema" (a galeria de barras) vira **Estilo da barra**; **Temas** é o grupo novo, o primeiro de Aparência do dock. Só textos: `settings.json` não muda. |
| Q7, Q20 | A galeria nasce com **Padrão** e combos do que já existe; os de skin chegam com as skins. |
| Q8 | Um tema não mexe no modo de cor. A barra dos temas gamer é escura nos dois modos. |
| Q9 | Os widgets ficam como estão nesta leva. |
| Q10, Q14, Q21 | A barra dos temas gamer é uma **skin de 9 fatias** (PNG + `skin.json`), embutida, num formato pronto para uma pasta `Skins` no futuro. Com skin, somem só os ajustes que a imagem define (cantos, borda). |
| Q11 | Aplicar um tema troca a cor do indicador pela dele (`null`: a do estilo). |
| Q15 | Os temas são uma lista fixa no código (`src/core/themes.ts`). |
| Q16–Q18 | Os três temas gamer (§4). |
| Q19 | As skins e os pacotes dos temas aparecem também avulsos nas galerias. |
| Q13, Q22 | Ordem: (1) nomes e galeria com combos; (2) glifos dos launchers; (3) skin de 9 fatias, com Retrô, Clássico 98 e Fósforo; (4) Tático; (5) Medieval 8-bit; (6) Caçador. Um PR por passo, com folha comparativa e teste ao vivo. |

## 2. Passo 1: a galeria de Temas

- `src/core/themes.ts`: `THEMES`, a lista dos temas (`id`, `style`, `iconPack`, `indicatorColor`); `choicesOf(theme)` dá as três escolhas, e `themeOf(dock)` dá o tema em uso, ou `null` ("Personalizado"). O pacote e a cor se comparam sem diferenciar maiúsculas.
- Os combos de hoje:

| Tema | Estilo da barra | Ícones | Indicador |
|---|---|---|---|
| Padrão | Nativo | originais | do estilo |
| Neon | Neon | Neon | do estilo |
| Esmalte | Nativo | Esmalte | do estilo |
| Vidro | Liquid Glass | Vidro | do estilo |
| Meia-noite | Meia-noite | Traço | do estilo |

- Configurações › Aparência do dock ganha o grupo **Temas**, o primeiro: um cartão largo por tema (`ChoiceCards`, como os estilos), com o dock em miniatura no estilo do tema, os cinco primeiros apps fixados nos ícones dele (`icon_pack_preview`) e o indicador. Escolher um cartão grava as três escolhas de uma vez. Nenhum cartão marcado quando não há tema em uso, com a linha "Personalizado: …" embaixo. Sem apps fixados (o painel), a miniatura mostra os ladrilhos dos widgets.
- **Ícones na barra escura** (Q8): as imagens escuras de um pacote (`dark` do `pack.json`) seguem a barra, não só o modo de cor. `DockStyle::bar_dark(modo)`: Meia-noite, Rocket e Neon são barras escuras nos dois modos, Papel é clara nos dois, e as outras seguem o modo. Valem para o dock nativo, o `app_icon` e o `icon_pack_preview`, que recebe o estilo da miniatura (o do tema; o atual quando nulo). Sem isso, Meia-noite + Traço no modo claro punha o grafite do Traço sobre a barra preta.
- Textos: "Modo de cor", "Estilo da barra", "Personalizar este estilo" e "Temas", em português e inglês. O lançador acha Aparência também por "temas" e "estilo da barra", e Configurações por "modo de cor".
- Um pacote que o usuário apagou de `IconPacks` continua no tema: aplicado, o dock mostra os ícones originais e o cartão de ícones o mostra como indisponível, como hoje.
- **Testes:** `themes.test.ts` (cada tema usa um estilo que existe e um pacote embutido que existe; cada tema é reconhecido pelas suas escolhas; nenhum quando uma escolha é do usuário); `settings::bar_dark` (as barras escuras, a clara e as que seguem o modo); `ThemeGallery.test.tsx` (ordem dos cartões, tema em uso marcado, aplicar grava as três escolhas, "Personalizado", miniatura com os ícones do tema, desligado).

- **Verificado ao vivo** (2026-10-30, build de teste e2e, cópia dos dados do autor, modo claro): Aparência mostrou o grupo **Temas** com os 5 cartões e o **Estilo da barra**. Os dados do autor (Flutuante com indicador branco) apareceram sem tema marcado, com "Personalizado". Cada cartão gravou as três escolhas e ficou marcado, e o dock trocou barra e ícones na hora. A Meia-noite mostrou o Traço claro na barra preta, e o Neon os ícones soltos, como pede a barra escura. Trocar o pacote para Argila desmarcou o tema e mostrou "Personalizado". Memória: 295 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-temas-ao-vivo.png`](../fotos/g7-temas-ao-vivo.png).

## 3. A skin de 9 fatias (passo 3)

- **Formato:** `src-tauri/skins/<id>/skin.png` e `skin.json`, com o nome, o tamanho da imagem, o corte de 9 fatias (`slice`: `top`, `right`, `bottom`, `left`, em pixels), `middle` (`repeat` ou `stretch`) e `pixelated`. As fatias de cima e de baixo são vazias: a imagem inteira vai na altura da barra, então o corte vira três colunas (as pontas como estão e o meio repetido ou esticado até a largura). Um teste confere que o `skin.json` bate com o tamanho do PNG.
- **Fonte:** `src-tauri/skins/fonte/<id>.txt`, um desenho em texto com paleta, como os pacotes de pixel. `python gerar.py <id>` escreve os dois arquivos.
- **Dock nativo:** `DockStyle` ganha `Retro`, `Classic98` e `Phosphor`. Neles o `Look` tem `skin: true` e cores próprias de separador, tinta do relógio e indicador. A barra é desenhada por `skin_bar` (`paint.rs`): o bitmap da skin, feito uma vez, em pedaços calculados por `domain::skin::pieces`, sem suavização (pixel art) e com bordas duras, para os ladrilhos do meio não mostrarem emenda. Não há preenchimento, borda de luz, contorno nem brilho.
- **Miniatura:** `STYLE_PAINTS` lê os mesmos `skin.json` e `skin.png` e desenha a barra com `border-image` (pontas e meio) e `image-rendering: pixelated`.
- **Ajustes (Q21):** numa skin, "Borda" e "Cantos" somem; "Reflexos" fica.
- **Barra escura:** Retrô e Fósforo são escuras nos dois modos; o Clássico 98 é claro nos dois.
- **As três skins** (16 px de altura, ×4 numa barra de 64 px):

| Skin | Desenho | Indicador | Tema |
|---|---|---|---|
| Retrô | placa azul-noite PICO-8, contorno preto, luz em cima e à esquerda, cantos de pixel | `#FF004D` | Retrô (com o pacote Retrô 8-bit) |
| Clássico 98 | painel elevado do Windows 98 (branco, cinza-claro, cinza, preto) | `#000080` | Clássico 98 (com o pacote Clássico 98) |
| Fósforo | moldura com bisel e tela escura com linhas de varredura | `#3CFF6E` | Fósforo (com o pacote Fósforo) |

- **Testes:** `domain::skin` (as pontas na altura da barra; o meio em ladrilhos com o último cortado; a barra estreita encolhe as pontas; cada skin embutida bate com o PNG e com o estilo); `look`, `settings` (os estilos novos no JSON e na barra escura), `paints.test` (a miniatura marca a skin que `look.rs` marca), `StyleGallery` (a barra de skin com a imagem dela), `LookPanel` (sem borda e cantos numa skin) e `ThemeGallery` (os 8 temas).
- **Verificado ao vivo** (2026-10-30, build de teste e2e, cópia dos dados do autor): Padrão, Retrô, Clássico 98 e Fósforo, aplicados pelos cartões, gravaram estilo e pacote, e o dock desenhou as skins. "Cantos" sumiu nas três. A primeira rodada mostrou finas emendas entre os ladrilhos do meio do Retrô e do Clássico 98. A correção: meio esticado nas skins de meio liso e bordas duras no desenho da skin. Na segunda rodada não houve emenda, nem no Fósforo, que repete o meio. Memória: 296 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-skins-ao-vivo.png`](../fotos/g7-skins-ao-vivo.png).

## 4. Os temas gamer (passos 4–6)

- **Tático** (passo 4, feito):
  - **Skin** `tactical` (`skins/fonte/tactical.py`, em vetor): 52×64 px, pontas de 24 px e meio esticado, suavizada (`pixelated: false`). Corpo chanfrado a 45° em gradiente carvão-oliva (`#2D3125` → `#191B13`), borda `#4A5038`, fio de areia (`#D9C9A3`) a 5,5 px do topo e colchetes de mira nas pontas. Separador areia a 30%, relógio areia, indicador laranja de alerta `#FF8A1F`. Barra escura nos dois modos.
  - **Pacote** `tatico` (`fonte/tatico.txt`, estilo de camadas `tatico`): o glifo de contorno em areia sobre placa octogonal (quadrado chanfrado em 29% do lado) em gradiente oliva (`#353A2A` → `#20241A`), com fio de areia a 40% na borda e sombra curta. A reserva é uma retícula de mira (`apps/mira.png`): o `vetor.py` passa a aceitar `reserva-id` e `reserva` no `.txt` do pacote, para trocar o glifo da reserva.
  - **Tema** `tactical`: skin Tático, pacote Tático e indicador do estilo.
  - **Verificado ao vivo** (2026-10-30, build e2e, cópia dos dados do autor): o cartão Tático gravou estilo `tactical` e pacote `tatico`, e o dock desenhou a barra chanfrada com o fio de areia, os ícones nas placas octogonais e as marcas laranja. "Cantos" sumiu. Memória: 294 MB em 7 processos com Configurações aberta. A barra do Windows ficou igual. Fotos em [`docs/fotos/g7-tatico-ao-vivo.png`](../fotos/g7-tatico-ao-vivo.png).
  - Com os glifos dos launchers (passo 2), o pacote cobre 55 apps, e o teste dos launchers inclui o Tático.
- **Medieval 8-bit** (passo 5, feito):
  - **Skin** `medieval` (`skins/fonte/medieval.txt`, pixel art 16×16): moldura de madeira escura com cantoneiras de ferro e rebites nas pontas (4 px cada) e o meio de tijolos de pedra, que se repete (8 px). Separador madeira, relógio pergaminho `#E8D9B0`, indicador ouro `#E8B84A`. Barra escura nos dois modos.
  - **Pacote** `medieval` (`fonte/medieval.txt`, pixel art 32×32, `gerar.py`): cada app num espaço de inventário de pedra com bisel. O objeto sai do glifo, com contorno escuro, corpo na cor terrosa mais próxima da matiz do app (vermelho, laranja, ouro, verde, verde-água, azul, roxo, vinho ou marrom), detalhes em pergaminho e sombra de 1 px. Um glifo sem área fechada leva o traço na cor do app. Cobre os 55 glifos. A reserva é um baú (`apps/bau.png`). Os desenhos partiram dos glifos e se editam no texto.
  - **Tema** `medieval`: skin Medieval, pacote Medieval e indicador do estilo.
- **Caçador:** barra de couro curtido com costura pespontada e cantos de osso; indicador âmbar `#F2A33A`. Ícones: medalhão de couro com aro de osso e o glifo gravado em âmbar, em relevo. Reserva: pegada de garra.
