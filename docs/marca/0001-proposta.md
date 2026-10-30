# Marca 0001 — Nome e logo (recorte G6)

Recorte G6 da [spec 0019](../specs/0019-fase-g-extras.md). Só proposta: o código continua DockMax até um recorte de renome.

**Status:** decidido pelo autor em 2026-10-29: o nome passa a ser **Dockyard** e a logo é o conceito **B (lançamento)** em esmalte **cobalto**.

**Atualização em 2026-10-30:** depois de quatro rodadas de desenho, o ícone do app passou a ser o **D + Salto** (desenho 7) em cobalto, no lugar do B, e o ícone da bandeja é a versão **monocromática** do mesmo glifo. Veja a seção 3.1.

## 1. Decisão

| | |
|---|---|
| Nome | **Dockyard** |
| Ícone | [`dockyard-icone.svg`](dockyard-icone.svg): o **D + Salto** (desde 2026-10-30). A haste do D é a barra do dock e o ladrilho do meio saltou para dentro do bojo. Para 16/20/24 px, [`dockyard-icone-16.svg`](dockyard-icone-16.svg), com a haste sólida e partida |
| Bandeja | [`dockyard-bandeja-clara.svg`](dockyard-bandeja-clara.svg) (glifo preto, barra clara) e [`dockyard-bandeja-escura.svg`](dockyard-bandeja-escura.svg) (glifo branco, barra escura): só o glifo, sem esquadro, como os ícones de rede e volume |
| Logotipo | [`dockyard-logotipo.svg`](dockyard-logotipo.svg) (fundo claro) e [`dockyard-logotipo-escuro.svg`](dockyard-logotipo-escuro.svg) (fundo escuro) |
| Esmalte | Cobalto, `hsl(224 58% 52%)` → `hsl(224 58% 34%)`, da família do destaque do app (`#2f5bd3`) |

Por que Dockyard: estaleiro é onde as coisas são montadas e de onde são lançadas, então o nome cobre o dock e o lançador sem precisar de um segundo termo. Mantém "dock" por dentro sem ser genérico, e lê do mesmo jeito em português e em inglês.

## 2. Como chegamos aqui

Entrevista com o autor, em rodadas:

1. **Público:** só ele e os amigos. Conflito de marca pesa pouco; não há venda nem registro de marca em vista.
2. **Tom:** técnico. **Idioma:** neutro entre PT e EN. **Herança:** liberdade total, sem obrigação de manter "Dock" ou "Max".
3. **Nomes recusados:** Tessera, Hangar, Pier, Kade, Strata, Pylon, Nexo; Vertex, Radix, Helix, Apex, Onyx, Codex, Axo, Nodo; TileBar, EdgeBar, AppRail, ShelfOS, WinDeck, Baseline, LaunchDock. Do primeiro lote só **Axis** agradou.
4. **Direção escolhida:** composto técnico, no estilo de PowerToys e FancyZones. O autor lembrou que o nome precisa cobrir o dock **e** o lançador.
5. **Checagem de conflito** (Axis, Dockyard, DockKit, DockRun), abaixo.
6. **Finalistas para a logo:** Dockyard, com DockMax como referência. Três conceitos; o autor escolheu Dockyard B e depois o esmalte cobalto entre tinta, petróleo, cobalto e musgo.

### Checagem de conflito (2026-10-29)

| Nome | Microsoft Store (busca BR) | `.app` / `.dev` (RDAP) | GitHub e outros |
|---|---|---|---|
| **Dockyard** | nenhum app com o nome | os dois registrados | DockYard é uma consultoria conhecida (Elixir/Ember); `ropali/dockyard` é um cliente de Docker (322★). Nenhum dock ou lançador de Windows com o nome |
| Axis | nenhum app com o nome | os dois registrados | marca forte de terceiros (Axis Communications, câmeras; Axis Bank); busca ruidosa |
| DockKit | nenhum app com o nome | os dois livres | é o nome de um framework da Apple (iOS 17, suportes motorizados) |
| DockRun | nenhum app com o nome | `.dev` livre, `.app` registrado | wrappers pequenos de `docker run`; o nome lê como "docker run" |

Para o público decidido (o autor e os amigos), nenhum conflito bloqueia o Dockyard. Antes de publicar na Store ou abrir para o público, refazer a checagem e escolher um domínio (os de `dockyard.app` e `dockyard.dev` estão tomados; o projeto pode seguir em `maxsueleinstein.dev`).

## 3. Conceitos

Todos em [`conceitos/`](conceitos/). A página [`conceitos/conceitos.html`](conceitos/conceitos.html) compara os três de 256 a 24 px, no claro e no escuro, e [`conceitos/variantes.html`](conceitos/variantes.html) mostra o B nos quatro esmaltes. Para abrir as páginas com as imagens, sirva a pasta (`python -m http.server` dentro de `docs/marca/conceitos`); pelo `file://` o navegador do app não carrega os SVGs.

| Conceito | Ideia | Resultado |
|---|---|---|
| Dockyard A · Pórtico | Pórtico de estaleiro erguendo um ladrilho da barra | Metáfora mais literal; fica poluído a 24 px |
| **Dockyard B · Lançamento** | O prompt do lançador saindo da barra | **Escolhido**, em cobalto |
| DockMax · Ampliação | Ladrilhos crescendo em curva até o do meio, como a ampliação | Referência, caso o nome não mudasse |

Os SVGs seguem o DESIGN.md: superelipse com esmalte de dois tons da mesma matiz, brilho no topo, fio de luz e glifo branco. O logotipo usa Segoe UI Variable Display 600 como texto; **antes de usar fora do repositório, converter o texto em contorno**, porque a fonte não existe fora do Windows.

### 3.1. Ícone D + Salto (2026-10-30)

O ícone do conceito B (o prompt saindo da barra) não agradou no tamanho real. Mais quatro rodadas:

1. Seis ideias novas (doca, letra D, pilha, guindaste, âncora, salto).
2. Oito variações das que sobraram.
3. Confronto do **7 · D + Salto** com o **8 · Doca + Salto**, em contexto e tamanho real: bandeja, barra de título, barra de tarefas, menu Iniciar, dock e 256 px ([`conceitos/confronto-d-salto.png`](conceitos/confronto-d-salto.png)). **Escolhido o 7**, em cobalto.
4. Três opções para a bandeja: colorido, monocromático e contorno ([`conceitos/bandeja-opcoes.png`](conceitos/bandeja-opcoes.png)). **Escolhido o monocromático (B)**: preto na barra clara, branco na escura.

No app:

- `src-tauri/icons/` foi gerado com `pnpm tauri icon docs/marca/dockyard-icone.svg`. O `icon.ico` foi montado à mão: 16, 20 e 24 px vêm de `dockyard-icone-16.svg`; 32, 40, 48, 64 e 256 px, de `dockyard-icone.svg`, cada um rasterizado direto do SVG no seu tamanho.
- A bandeja (`src-tauri/src/tray.rs`) embute os PNGs de `src-tauri/icons/tray/` (preto e branco, a 16, 20, 24 e 32 px). A cor segue o tema da **barra de tarefas** (`SystemUsesLightTheme`), não o dos apps; o tamanho segue a escala do monitor principal.
- As cores `hsl()` dos SVGs foram gravadas em hex (`#4267cd` → `#2a499d`, a mesma cor), para qualquer rasterizador ler.
- Os logotipos (`dockyard-logotipo*.svg`) ainda trazem o símbolo do B e ficam para outro recorte.

## 4. O que o renome exige no código (recorte futuro)

Recomendação: fazer em duas etapas. A **R1** troca tudo o que se vê e mantém o identificador e os caminhos de dados, então não há migração. A **R2** é opcional e só vale se o identificador antigo incomodar.

### R1. Renome visível (sem migração de dados)

**Status:** feita em 2026-10-30 (branch `feat/r1-dockyard`). Ícones gerados de `dockyard-icone.svg`; `productName` Dockyard; exe `dockyard.exe` e lib `dockyard_lib`; vigia `dockyard-guard.exe`, que apaga a cópia `dockmax-guard.exe` que sobrou na pasta de dados (um vigia antigo em execução não é tocado: ele termina sozinho com a execução que vigia); entrada `Dockyard` no `Run`, e a primeira execução de release troca a `DockMax` por ela, levando junto o estado ligado/desligado de "Aplicativos de inicialização"; o desinstalador apaga as duas entradas e os dois vigias. O instalador não remove uma instalação antiga do DockMax (outra pasta e outra chave de desinstalação): ela fica ao lado até o usuário desinstalá-la. Classes de janela, IDs de automação e prefixos de arquivos temporários ficaram com o nome antigo.

1. **Ícones do app:** gerar os PNG, `.ico` e `.icns` de `src-tauri/icons/` a partir de `dockyard-icone.svg` (`pnpm tauri icon docs/marca/dockyard-icone.svg`). Hoje eles ainda são o ícone padrão do Tauri. Conferir a 16 e 24 px; se o glifo sumir, fazer uma versão simplificada só para os tamanhos pequenos (sem as linhas de impulso).
2. **Nome do produto:** `productName` em `src-tauri/tauri.conf.json` → `Dockyard`. Isso muda a pasta de instalação do NSIS, o nome no menu Iniciar e na lista de apps instalados. **Verificar** como o instalador do Tauri trata a versão já instalada com outro `productName` (a chave de desinstalação pode mudar); se instalar ao lado, o `windows/hooks.nsh` precisa remover a instalação antiga do DockMax.
3. **Nome do exe:** `[package] name` em `src-tauri/Cargo.toml` (`dockmax` → `dockyard`), `dockmax_lib` → `dockyard_lib` (e o alvo do log em `logging.rs`), `name` em `package.json`. O vigia da barra (`dockmax-guard.exe`, `taskbar.rs`) troca de nome junto; o vigia antigo precisa continuar sendo encontrado e encerrado na primeira execução, para a barra do Windows sempre voltar.
4. **Iniciar com o Windows:** a entrada `DockMax` na chave `Run` (`autostart.rs`) aponta para o exe antigo. Na primeira execução: se a entrada antiga existir, apagá-la e criar `Dockyard` com o caminho novo.
5. **Textos:** `src/i18n/pt-BR.ts`, `src/i18n/en.ts` e `src-tauri/src/i18n.rs` (nomes no dock, menus, lançador, "Sobre"), as mensagens do login do Google (`google.rs`), notificações (`toast.rs`), `README.md`, `PRODUCT.md` (Brand Commitments) e `DESIGN.md` (título do sistema). As specs e ADRs antigos ficam como estão: são histórico.
6. **Nomes de classe das janelas** (`DockMaxDock`, `DockMaxAppBar`, `DockMaxLauncher`, `DockMaxLauncherList`): podem ficar; o usuário não os vê. Trocar só se for trivial, e procurar antes os `FindWindow` que buscam essas classes (por exemplo em `native_dock/mod.rs`).
7. **Testes e e2e:** asserts com `dockmax` (`logging.rs`, `autostart.rs`, `taskbar.rs`, `e2e.rs`, `e2e/specs`) acompanham o que mudar.

Ficam como estão na R1: o identificador `dev.maxsueleinstein.dockmax`, as pastas em `%APPDATA%` e `%LOCALAPPDATA%` derivadas dele, `dockmax.db` e os arquivos de log, as credenciais do Google no Gerenciador de Credenciais (`DockMax/google-client`, `DockMax/google:*`) e a variável `DOCKMAX_E2E_DATA_DIR`.

### R2. Identificador e dados (opcional)

1. `identifier` → `dev.maxsueleinstein.dockyard`. Os dados passam a viver em outra pasta.
2. **Migração na primeira execução:** se a pasta nova não existir e a antiga existir, copiar (não mover) `settings.json`, `dockmax.db` (renomeado para `dockyard.db`, com os `-wal`/`-shm` fechados antes) e os pacotes de ícones; deixar a pasta antiga intacta até uma versão seguinte, para dar para voltar. Registrar no log o que foi copiado.
3. Credenciais do Google: ler as entradas `DockMax/…`, regravar como `Dockyard/…` e só então apagar as antigas.
4. Atualização automática (G3): o canal de atualização precisa saber do identificador novo antes da troca, senão a versão instalada não acha a seguinte.
5. Testes de migração: pasta antiga com e sem banco, banco corrompido, credenciais ausentes, e a segunda execução não copiando de novo.

## 5. O que falta decidir

- Se a R2 entra, ou se o identificador antigo fica para sempre.
- O domínio, se um dia o projeto for publicado (os `dockyard.*` óbvios estão tomados).
- Quando fazer o recorte de renome em relação ao G3 (atualização automática): renomear antes de publicar a primeira versão com atualização evita migrar o canal depois.
