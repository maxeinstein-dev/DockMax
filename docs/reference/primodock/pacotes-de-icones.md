# Pacotes de ícones do PrimoDock

Base para a fase B4 (pacotes de ícones, ícone por item e imagem própria). Nenhum ícone ou miniatura foi copiado: só o formato e os nomes.

## O pacote instalado

Cada pacote é uma pasta em `%APPDATA%\PrimoDock\IconPacks\<id>\`, baixada como `.zip` da galeria. O instalado na máquina do autor é o **Pixel System** 1.2.0.

```
pixel-system\
  pack.json
  01-system\  02-files\  03-development\  04-internet\  05-productivity\
  06-creative\  07-ai\  08-security\  09-gaming\  10-utilities\  11-fallback\
  apps\        ícones de programas específicos, pelo nome do programa
  _widget\     t0.png ... t8.png, as bases dos widgets
```

As **11 categorias** são as pastas numeradas: Sistema, Arquivos, Desenvolvimento, Internet, Produtividade, Criação, IA, Segurança, Jogos, Utilitários e o ícone de reserva (`11-fallback`), usado quando nada casa. Os ícones são PNG (cerca de 5 KB cada no Pixel System).

## `pack.json`

| Campo | Valor visto | Papel |
|---|---|---|
| `name`, `author`, `version` | `"Pixel System"`, `"PrimoDev"`, `"1.2.0"` | identificação |
| `description` | `{ "en": "...", "pt-BR": "..." }` | texto da galeria, por idioma |
| `publishedAt`, `modifiedAt` | datas ISO 8601 | versões |
| `order` | `11` | posição na galeria |
| `accent` | `"#3A63D6"` | cor do pacote na galeria |
| `previews` | 24 caminhos relativos | os ícones mostrados no cartão da galeria |
| `padding` | `0.07` | margem em volta do ícone, em fração do lado |
| `icons` | mapa de 371 entradas: **nome do programa → arquivo** | qual arquivo vale para cada programa |
| `widget` | objeto (abaixo) | como os widgets se vestem com este pacote |

**`icons`**: a chave é o nome pelo qual o programa é reconhecido, e o mesmo arquivo aparece com várias chaves (o nome de exibição e o do executável: `Inkscape` e `inkscape`, `LibreOffice` e `soffice`; e até nomes de apps do macOS, como `Finder` para o Explorador de Arquivos e `System Settings` para Configurações, herança de um produto irmão no Mac). 318 entradas apontam para `apps\`, o resto para as categorias. A lista de nomes está em [apps-com-icone-proprio.txt](apps-com-icone-proprio.txt).

**`widget`**:

| Campo | Valor visto | Papel |
|---|---|---|
| `face`, `bounds` | `[0.0312, 0.0312, 0.9375, 0.9375]` | retângulo do mostrador e da peça, em fração do ladrilho |
| `slice` | `0.18` | fatia de nove partes da base (para esticar em ladrilhos largos) |
| `effect`, `font`, `weight` | `"flat"`, `"mono"`, `"bold"` | acabamento e fonte dos números |
| `tone` | `"t0"` | tom padrão |
| `tones` | nove tons, `t0` a `t8`, cada um com `base` (a imagem em `_widget\`), `ink`, `secondary`, `accent` e três `surfaces` | as paletas |
| `kinds` | tipo de widget → tom | qual tom cada widget usa |

Tons do Pixel System (`surfaces` de cada um): t0 azul `#3A63D6`, t1 laranja `#E5813A`, t2 grafite `#363846`, t3 verde-água `#1E9A93`, t4 vermelho `#D3493E`, t5 magenta `#B7458C`, t6 roxo `#6C4ECF`, t7 creme `#E6D9B8`, t8 azul-acinzentado `#5D6D85`. Os 31 tipos de widget e seus tons estão em [settings-e-dados.md](settings-e-dados.md).

## Como um programa ganha ícone

Pelo `pack.json` e pelo catálogo, a busca é: o nome do programa (de exibição ou do executável) nas chaves de `icons`; senão, a **chave semântica** do programa (o catálogo traz, por pacote, a lista de chaves que ele cobre: `browser`, `terminal`, `aicoding`, `analytics`...) na categoria; senão, o ícone de reserva. São 649 chaves no Pixel System (os pacotes Primo, mais antigos, cobrem 265); a lista está em [chaves-de-icone.txt](chaves-de-icone.txt). Para o DockMax, essa lista é um bom ponto de partida para a taxonomia própria de ícones da B4.

## Catálogo da galeria

`icon-gallery-catalog.json` tem `items`, um por pacote, com `id`, `name`, `author`, `summary` (por idioma), `version`, `publishedAt`, `modifiedAt`, `iconCount`, `accent`, `previews` (24 caminhos no servidor), `source` (o `.zip` num serviço da própria empresa), `keys` (as chaves que o pacote cobre), `sha256` e `size`. `icon-gallery-seen.json` é a lista de ids que o usuário já viu (para marcar novidade).

Os 14 pacotes do catálogo em 2026-10-25:

| id | Nome | Versão | Ícones | Chaves | Destaque | Tamanho |
|---|---|---|---|---|---|---|
| `primo` | Primo | 1.1.0 | 207 | 265 | `#2563EB` | 19,8 MB |
| `primo-sunset` | Primo Sunset | 1.1.0 | 207 | 265 | `#EA580C` | 23,1 MB |
| `primo-mono` | Primo Mono | 1.1.0 | 207 | 265 | `#52525B` | 12,8 MB |
| `primo-midnight` | Primo Midnight | 1.1.0 | 207 | 265 | `#312E81` | 18,7 MB |
| `mono-pro` | Mono Pro | 1.2.0 | 358 | 643 | `#18181B` | 5,5 MB |
| `obsidian-glass` | Obsidian Glass | 1.2.0 | 364 | 649 | `#1E2430` | 35,2 MB |
| `soft-clay` | Soft Clay | 1.2.0 | 362 | 647 | `#E8644E` | 32,5 MB |
| `retro-future` | Retro Future | 1.2.0 | 364 | 649 | `#F2992B` | 25,2 MB |
| `swiss-flat` | Swiss Flat | 1.2.0 | 364 | 649 | `#2350D7` | 3,2 MB |
| `neon-grid` | Neon Grid | 1.2.0 | 364 | 649 | `#FF3FCF` | 32,3 MB |
| `pixel-system` | Pixel System | 1.2.0 | 364 | 649 | `#3A63D6` | 0,6 MB |
| `acrylic-spectrum` | Acrylic Spectrum | 1.2.0 | 364 | 649 | `#9479FF` | 51,9 MB |
| `halloween` | Halloween | 1.0.0 | 364 | 649 | `#FF7A00` | 29,6 MB |
| `christmas` | Christmas | 1.0.0 | 360 | 645 | `#C8283A` | 28,2 MB |

## Notas para o DockMax

- O DockMax não usa nem baixa estes pacotes (são ativos de terceiros). A B4 terá pacotes próprios, num formato inspirado neste: um manifesto com metadados e destaque, ícones por categoria e por programa, um ícone de reserva, e paletas para os widgets.
- O que vale reaproveitar como ideia: a busca em três passos (programa, chave semântica, reserva), o `padding` por pacote, as paletas dos widgets por tipo e o `sha256` para conferir o download.
