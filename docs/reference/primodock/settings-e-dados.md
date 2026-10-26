# Configurações e dados do PrimoDock

Pasta de dados: `%APPDATA%\PrimoDock`. Todos os arquivos são JSON, escritos com os nomes de campo em `camelCase`. Valores abaixo são os vistos na instalação do autor; onde há uma lista, é a lista completa observada.

| Arquivo | Papel |
|---|---|
| `settings.json` | preferências do dock e do app |
| `workspaces.json` | espaços de trabalho, itens, widgets e monitores |
| `session.json` | de que espaço de trabalho cada janela aberta é |
| `icons.json` | pacote de ícones padrão e trocas por item |
| `IconPacks\<id>\` | pacotes de ícones instalados ([pacotes-de-icones.md](pacotes-de-icones.md)) |
| `icon-gallery-catalog.json`, `icon-gallery-seen.json` | catálogo da galeria e o que o usuário já viu |
| `primodock.log` | registro de execução ([comportamentos.md](comportamentos.md)) |

## `settings.json`

Raiz:

| Campo | Tipo e valor visto | Papel |
|---|---|---|
| `taskbar` | `"hidden"` (as três opções da tela Geral: visível, automática e oculta) | o que o PrimoDock faz com a barra do Windows |
| `taskbarAutoHideBefore` | `false` | se a barra estava em ocultar automaticamente antes do PrimoDock: para devolvê-la como estava |
| `onboardingSeen` | número (`2`) | versão do tour já vista |
| `defaultsVersion` | número (`1`) | versão dos padrões já aplicados, para migrar |
| `widgetsFollowIconPack` | `true` | os widgets usam os tons do pacote de ícones |
| `language` | `"system"` | idioma da interface |
| `petEnabled`, `petSpecies`, `petCoat` | `true`, `"cat"`, `"cream"` | o mascote |
| `dock` | objeto | tudo do dock |

`dock`:

| Campo | Tipo e valor visto | Papel |
|---|---|---|
| `themeId` | `"floating"` | tema da barra; ids vistos: `native`, `liquid`, `midnight`, `rocket`, `aqua`, `neon`, `canarinho`, `paper`, `floating` |
| `themeCustomizations` | mapa de id de tema para objeto (abaixo) | um objeto por tema, com todos os campos `null` até o usuário mudar |
| `position` | `"bottom"` | borda: `bottom`, `top`, `left`, `right` |
| `iconSize` | `52` | tamanho do ícone em pixels |
| `bottomSpace` | `2` | distância até a borda da tela, em pixels |
| `iconShape` | `"plates"` | `plates`, e (pelas telas) mesmo tamanho e como vieram |
| `hoverEffect` | `"scale"` | `none`, `scale`, `magnify` |
| `hoverScale` | `1.1` | fator da escala (1,00 a 1,60 nas telas) |
| `launchAnimation` | `"bounce"` | `bounce`, pulso e tremor |
| `indicator` | `"dot"` | ponto ou traço |
| `indicatorFollowsWorkspace` | `false` | a cor do indicador é a do espaço de trabalho |
| `indicatorColorHex` | `"#FFFFFF"` | cor do indicador |
| `showsRunningApps` | `true` | mostra os apps abertos não fixados |
| `showsWorkspaceSwitcher` | `true` | as bolinhas de espaço de trabalho |
| `showsSystemStatus` | `true` | bateria, rede e volume no fim do dock |
| `showsClock`, `showsDate` | `true`, `true` | relógio e data |
| `showsTrayIcons` | `true` | ícones da bandeja |
| `trayIconsInDock` | lista de chaves de ícone | quais ícones da bandeja ficam fixados no dock |
| `hiddenTrayIcons` | lista (vazia) | ícones escondidos |
| `showsLauncher` | `true` | o item "lançador" |
| `launcherOpensWindowsStart` | `true` | o lançador abre o Iniciar do Windows |
| `autoHide` | `false` | ocultar o dock automaticamente |
| `keepsWindowsOff` | `true` | manter as janelas fora da área do dock |
| `revealDelay` | `0.5` | segundos até revelar, com ocultar automático |
| `onEveryDisplay` | `true` | um dock em cada monitor |
| `showsPreviews` | `true` | prévias de janela |
| `previewDelay` | `0.2` | segundos até a prévia |
| `previewsOnlyWithSeveralWindows` | `false` | prévia só com várias janelas |
| `savesEnergy` | `false` | modo de economia de energia |

**Chave de ícone da bandeja** (`trayIconsInDock`): o texto `path:` seguido do caminho do executável **em minúsculas** e `#` e um número (o identificador do ícone daquele programa): `path:c:\program files\app\app.exe#3`. Aplicativos da Loja têm a versão do pacote trocada por `*`, para sobreviver a atualizações.

**`themeCustomizations.<tema>`**, todos `null` (o padrão do tema) até o usuário mudar:

| Campo | Papel |
|---|---|
| `topHex`, `bottomHex` | cor de cima e de baixo da barra |
| `opacity` | opacidade |
| `showsBorder`, `borderHex`, `borderWidth` | borda: liga, cor e espessura |
| `showsGlow`, `glowHex` | brilho: liga e cor |
| `cornerRadiusScale` | cantos, de 0 a 1 (as telas mostram 0 a 100 %) |
| `showsReflections` | reflexos |
| `separatorHex` | cor do separador |
| `showsSnowfall`, `showsSnowCap`, `showsLights` | os temas sazonais: neve, capa de neve e luzes |

O autor tinha mudanças em `liquid` (sem borda, cantos 0,5, sem reflexo) e em `floating` (sem borda).

## `workspaces.json`

```
{
  "version": 1,
  "activeId": "<id do espaço ativo>",
  "workspaces": [
    { "id": "<uuid>", "name": "Work", "colorHex": "#0A84FF",
      "items": [ { "kind": "app" | "widget", "target": "...", "title": null }, ... ],
      "stacks": [] }
  ],
  "widgets": [ { "id": "<uuid>", "name": "...", "kind": "...", "span": 2,
                 "settings": { ... }, "maxSpan": 4 } ],
  "displays": { "\\\\.\\DISPLAY1": "<id do espaço>", ... }
}
```

- **Espaços de trabalho:** `id` UUID, `name` livre, `colorHex` (os três de fábrica: Work `#0A84FF`, Personal `#FF9F0A`, Focus `#30D158`, que as telas mostram como Trabalho, Pessoal e Foco). `stacks` guarda os widgets empilhados.
- **Itens** (`items`): `kind` é `app` ou `widget`. Em `app`, `target` é o caminho do executável, o AUMID de um app da Loja ou um valor especial: `|launcher` é o lançador. Em `widget`, `target` é o id do widget. `title` é um nome próprio opcional (`null` = o do programa).
- **Widgets** ficam numa lista única, fora dos espaços: um espaço os mostra pelos itens `widget`. `span` é a largura em ícones e `maxSpan` o máximo que o widget aceita (por exemplo `activity` 2 de 4, `network` 2 de 2).
- **Monitores** (`displays`): o nome do monitor aponta para o espaço de trabalho ativo nele. Cada monitor pode estar em um espaço diferente.

**Tipos de widget** (`kind`), 31, na ordem do pacote de ícones: `activity`, `airdrop`, `battery`, `bookmarks`, `calendar`, `claudeUsage`, `clipboard`, `clock`, `collection`, `colourPicker`, `converter`, `countdown`, `emoji`, `fans`, `files`, `github`, `keyboardLock`, `music`, `network`, `note`, `pet`, `pomodoro`, `reminders`, `screenshots`, `shelf`, `shortcut`, `stopwatch`, `timeProgress`, `timer`, `water`, `weather`. Nem todos estão no catálogo da tela de widgets de hoje (a spec 0004 §3.5 lista os que apareceram nas capturas); os demais constam da tabela de tons do pacote e podem ser widgets previstos.

**`settings` de um widget** (todos os campos existem em todo widget; o que não vale para o tipo fica no padrão):

| Campo | Valor visto | Quem usa |
|---|---|---|
| `style` | `"numbers"` (e barras, anéis) | atividade, rede |
| `showsSeconds`, `isAnalog`, `timeZoneId` | `false`, `false`, `null` | relógio |
| `showsPercentage`, `showsArtwork`, `tintHex` | booleano, booleano, `null` | progresso, música |
| `presetMinutes` | lista | temporizador (predefinições) |
| `focusMinutes`, `shortBreakMinutes`, `longBreakMinutes`, `cyclesBeforeLongBreak` | números | Pomodoro |
| `text`, `colorHex` | texto, `null` | texto |
| `cityName`, `latitude`, `longitude`, `fahrenheit` | `null`, `null`, `null`, booleano | clima |
| `showsCpu`, `showsMemory` | booleanos | atividade do sistema |
| `scope` | `"day"` | progresso do tempo e contagem regressiva |
| `target` | `null` | contagem regressiva |
| `waterGoalMl`, `glassMl`, `waterDrunkMl`, `waterDay` | números e `null` | água |
| `claudeScope`, `tokenBudget` | `"block"`, `null` | uso do Claude |
| `codexShows`, `codexScope` | `"limit"`, `"block"` | uso do Codex |
| `openCodeScope`, `openCodeShows` | `"today"`, `"tokens"` | uso do OpenCode |
| `showsLayoutName` | booleano | teclado |
| `folderPath` | `null` | pasta |
| `todos` | lista | tarefas |

## `session.json`

`{ "Owners": { "<identificador da janela>": "<id do espaço de trabalho>" }, "Parked": { } }`: a que espaço de trabalho pertence cada janela aberta (o identificador é o número da janela no Windows) e as janelas "estacionadas", escondidas ao trocar de espaço. Serve para devolver as janelas ao espaço certo quando o dock reabre.

## `icons.json`

`{ "defaultPack": "pixel-system", "perItem": { }, "pictures": { } }`: o pacote padrão, a troca de pacote por item (o menu "Ícone" de um app) e as imagens próprias por item.

## Notas para o DockMax

- O DockMax guarda as mesmas coisas em lugares diferentes (as configurações de aparência em `settings.json` v1 aditivo, os apps fixados e os widgets no banco), e continua assim: o que importa daqui é o **conjunto de campos**, não o formato do arquivo.
- Campos que o DockMax já tem com outro nome: `themeId`→`style`, `position`→`edge`, `bottomSpace`, `iconShape` (plates, uniform, original), `hoverEffect`→`hover`, `launchAnimation`→`openAnimation`, `indicator`, `indicatorColorHex`→`indicatorColor`, `taskbar`.
- Ainda sem equivalente: tudo de espaços de trabalho, `showsRunningApps`, `autoHide`, `keepsWindowsOff`, `onEveryDisplay`, `showsPreviews`, `savesEnergy`, `petEnabled`, `trayIconsInDock`, `themeCustomizations` com cor, opacidade e brilho, e os temas sazonais.
