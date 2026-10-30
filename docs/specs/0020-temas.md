# Spec 0020 — Temas

Extensão da fase G ([spec 0019](0019-fase-g-extras.md)): **temas** que vestem o dock de uma vez, como os temas sazonais do PrimoDock, começando por temas gamer.

**Status:** em andamento: passo 1 (nomes, galeria de Temas, combos do que já existe e ícones que seguem a barra escura).

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

## 3. A skin de 9 fatias (passo 3)

A definir no PR dele: `skin.json` com nome, imagem, as quatro margens do corte e se o meio e as bordas esticam ou repetem; desenhada pelo dock nativo (Direct2D) e pela miniatura (`border-image`).

## 4. Os temas gamer (passos 4–6)

- **Tático:** barra carvão com toque de verde-oliva, cantos chanfrados a 45°, fio cor de areia no topo e colchetes de mira nas pontas; indicador laranja de alerta `#FF8A1F`. Ícones: glifo de contorno areia `#D9C9A3` numa placa octogonal verde-oliva escura com fio claro. Reserva: retícula.
- **Medieval 8-bit:** barra em pixel art, com moldura de madeira escura, cantoneiras de ferro com rebites e meio de pedra; indicador ouro `#E8B84A`. Ícones: pixel 32×32 em "espaço de inventário" de pedra com bisel, paleta terrosa limitada, fonte em texto e `gerar.py`. Reserva: baú.
- **Caçador:** barra de couro curtido com costura pespontada e cantos de osso; indicador âmbar `#F2A33A`. Ícones: medalhão de couro com aro de osso e o glifo gravado em âmbar, em relevo. Reserva: pegada de garra.
