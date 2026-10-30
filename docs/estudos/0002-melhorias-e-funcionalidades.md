# Estudo 0002 — Melhorias e funcionalidades

Levantamento de 2026-10-30, depois da primeira leva da [spec 0020](../specs/0020-temas.md) (temas). É só estudo: nada do app mudou. As fontes foram o código (`src-tauri/src` e `src`), as specs (sobretudo [0004](../specs/0004-paridade-primodock.md), [0019](../specs/0019-fase-g-extras.md) e [0020](../specs/0020-temas.md)), o [`DESIGN.md`](../../DESIGN.md), a [referência do PrimoDock](../reference/primodock/README.md), o [roteiro de verificação](../roteiro-de-verificacao.md) e os PRs até o #118. As linhas citadas são as do commit `c24b2ed`.

## Como ler

- **Melhorias** são ajustes no que já existe. Entram agora, na ordem da lista.
- **Funcionalidades** são coisas novas. Entram **depois** dos quatro itens já adiados da spec 0020, que não se repetem aqui: os temas Cyberpunk, Arcade e Sci-fi; os widgets no tom do tema (Q9); a pasta `Skins` de terceiros (Q14); e os glifos dos launchers nos pacotes de pixel.
- **Tamanho:** P (um PR pequeno, até um dia), M (um PR médio, alguns dias), G (uma spec própria e vários PRs).
- **Risco:** o que pode dar errado e o quanto mexe no que já funciona.
- A prioridade pesa, nesta ordem: memória e CPU em repouso (o dock fica aberto o dia todo), robustez (o dock substitui a barra do Windows), acessibilidade e o resto.

## Resumo

| # | Melhoria | Tamanho | Risco |
|---|---|---|---|
| M1 | Medir de novo a memória em repouso e pôr uma guarda no CI | M | Baixo |
| M2 | Podar os caches do dock nativo | P | Baixo |
| M3 | Menos despertares em repouso (mídia, status, selos, auto-ocultar, letreiro) | M | Médio |
| M4 | Um panic no dock não pode derrubar o app | M | Médio |
| M5 | Contraste do texto branco sobre o cobalto no escuro | P | Baixo |
| M6 | Alto contraste: foco visível no front e cores no dock nativo | M | Baixo |
| M7 | Leitor de tela nas escolhas e no "Manter este tema?" | P | Baixo |
| M8 | Painel e Configurações mais leves no WebView2 | M | Baixo |
| M9 | Documentos desatualizados | P | Nenhum |
| M10 | Testes que faltam no front | M | Baixo |
| M11 | Rodada de verificação ao vivo, por prioridade | M | Baixo |
| M12 | Dívida técnica: arquivos grandes e código repetido | G | Médio |
| M13 | Tokens do `DESIGN.md` que não viraram CSS | M | Baixo |
| M14 | Teclado no dock nativo | G | Médio |
| M15 | Lançador na escala nova ao trocar de monitor | P | Baixo |

| # | Funcionalidade (depois dos adiados da 0020) | Tamanho | Risco |
|---|---|---|---|
| F1 | Economia de bateria | M | Baixo |
| F2 | Espaçadores no dock | M | Baixo |
| F3 | Pilhas de widgets ("Empilhar com", Ctrl+roda) | M | Médio |
| F4 | Ícone por app (B4b) | M | Médio |
| F5 | Downloads, Lixeira e pastas no dock | M | Médio |
| F6 | Busca dentro de Configurações | P | Baixo |
| F7 | Pequenos do PrimoDock: "Procurar…", "Nova janela", "Duplicar espaço" | P | Baixo |
| F8 | Exportar e importar as configurações | M | Médio |
| F9 | Widgets que faltam do catálogo | M por widget | Baixo |
| F10 | Prévias: Espiar, cartão pelo teclado | M | Médio |
| F11 | Selos na bandeja e selos sem número | M | Médio |
| F12 | Dock vertical completo para widgets | M | Médio |
| F13 | Itens e espaço por monitor | G | Alto |
| F14 | Extensões do lançador | G | Alto |
| F15 | Temas sazonais | M | Baixo |

## Melhorias

### M1. Medir de novo a memória em repouso e pôr uma guarda no CI

- **O que é:** a última medição em repouso é a da 0.3.0 com o dock nativo: **64,5 MB** num processo só ([roadmap](../roadmap.md), "Medição de recursos"). Desde então entraram os espaços, as prévias, a bandeja, 17 widgets, o lançador com arquivos e área de transferência, a atualização automática, 12 pacotes e as skins. Os PRs recentes só medem com Configurações aberta (294–298 MB em 7 processos, #113–#117), o que diz pouco sobre o dia inteiro. Proposta: medir de novo em repouso (dock só, painel fechado há mais de 45 s) e depois de uma hora; e criar no e2e um caso que lê a memória privada do processo do dock com o painel fechado e falha acima de um teto (por exemplo, 90 MB).
- **Por que vale:** poupar memória é prioridade permanente, e hoje ninguém sabe se o repouso ainda está perto dos 65 MB. Uma guarda pega a regressão no PR que a causou.
- **Onde mexe:** `e2e/specs/` (um `memory.e2e.ts`), `e2e/app.ts`; um command de diagnóstico só no build `e2e` (o `windows/memory.rs` já existe); `docs/roadmap.md`.
- **Risco:** baixo. O cuidado é o teto não ficar instável no runner do GitHub (medir lá algumas vezes antes de fixar o número, com folga).
- **Como verificar:** a guarda falha com sabotagem (um `Vec` de 50 MB guardado no dock) e passa sem ela; a tabela do roadmap ganha a linha da 0.7.x.

### M2. Podar os caches do dock nativo

- **O que é:** quatro caches crescem o dia todo e nunca encolhem:
  - `icons` (`windows/native_dock/mod.rs:305`), com a chave pacote + programa + escuro: cada programa aberto que já passou pelo dock deixa um `ID2D1Bitmap`, e trocar de tema ou de modo de cor duplica os bitmaps. Só `forget_icons` (`mod.rs:3613`) limpa, e só quando um pacote muda;
  - `badges` (`mod.rs:306`), o mesmo padrão;
  - `tray_images` (`mod.rs:282`): só sai uma entrada quando o ícone perde a imagem (`mod.rs:971`), não quando o programa sai da bandeja;
  - `DESCRIPTIONS` (`windows/open_apps.rs:82`): sem teto, e guarda o `None` para sempre (um programa instalado depois fica sem descrição até reiniciar). Os caches irmãos já têm teto (`ICON_CACHE_LIMIT` em `commands/apps.rs:301`, `MAX_REMEMBERED` em `program_name.rs:37`).
- **Por que vale:** é memória que só sobe num processo que fica aberto por dias; o conserto é pequeno.
- **Onde mexe:** depois de cada `rebuild` do dock, um `retain` pelas chaves dos itens atuais e do pacote e tema atuais; em `decode_tray_images`, um `retain` pelas chaves da faixa; em `open_apps.rs`, um teto e o `None` com validade.
- **Risco:** baixo. Podar demais faz um ícone ser lido de novo (custa um pouco de CPU, não quebra nada).
- **Como verificar:** testes de unidade da poda (abrir e fechar 50 programas falsos deixa o mapa no tamanho dos itens atuais); ao vivo, a memória do dock depois de trocar de tema 10 vezes volta perto da inicial.

### M3. Menos despertares em repouso

- **O que é:** threads e timers que acordam mesmo sem nada para fazer:
  - a thread de mídia dorme 1 s e acorda para sempre, mesmo com `WANTED` falso (`windows/media.rs:262`);
  - o `STATUS_TIMER` lê bateria, rede e volume a cada 3 s (`mod.rs:100`), **um por dock**: com dois monitores são duas leituras, e cada uma cria um `MMDeviceEnumerator` e aloca 16 KB para `GetAdaptersAddresses` (`windows/system_status.rs:87-134`);
  - os selos leem a árvore de UI Automation da barra a cada 10 s mesmo sem eventos (`windows/badges.rs:54`, `:314`);
  - o auto-ocultar consulta o cursor a cada 100 ms (`mod.rs:97`);
  - o letreiro do "Tocando agora" repinta a ~30 fps a janela toda (`domain/dock_layout.rs:648`), e `paint()` só confere `visible`, não `tucked` (`mod.rs:1693`). Hipótese a confirmar: com o dock recolhido pelo auto-ocultar, o letreiro continua repintando.
- **Por que vale:** cada despertar custa pouco, mas o dock fica aberto o dia todo, inclusive na bateria. A medição de 0 % de CPU da 0.3.0 é de antes da maioria deles.
- **Onde mexe:** `media.rs` (um `Condvar` ou `park` enquanto ninguém quer); `system_status.rs` (uma leitura compartilhada por todos os docks; depois, `IAudioEndpointVolumeCallback` e `NotifyIpInterfaceChange` no lugar da consulta); `badges.rs` (sem batimento quando nenhum item tem selo); `native_dock/mod.rs` (não pintar o letreiro recolhido; consulta do cursor só perto da borda).
- **Risco:** médio. Trocar consulta por evento pode perder uma mudança (volume, rede) se o callback não chegar; manter uma consulta lenta de reserva (30 s).
- **Como verificar:** contar despertares com o Process Explorer (coluna "Context Switch Delta") antes e depois, com o dock parado por 5 minutos; testes das regras novas (uma leitura de status para N docks).

### M4. Um panic no dock não pode derrubar o app

- **O que é:** o perfil de release tem `panic = "abort"` (`src-tauri/Cargo.toml:130`) e as wndprocs do dock e do lançador são `extern "system"` sem `catch_unwind` (`native_dock/mod.rs:3409`; `launcher/mod.rs:2794`, `:2874`, `:2918`). Qualquer panic numa pintura derruba o processo; o vigia devolve a barra do Windows, mas o usuário fica sem dock até reabrir. Junto: `with_docks` descarta o trabalho em silêncio quando o `RefCell` já está emprestado (`mod.rs:557-559`), o que numa mensagem reentrante (menu modal) perde um evento sem deixar log; e as chamadas entre threads esperam até 5 s (`mod.rs:3720`, `:3753`).
- **Por que vale:** o dock substitui a barra do Windows; cair é o pior defeito possível. A ideia 3 da [referência](../reference/primodock/comportamentos.md) ("avisar e se recuperar quando o dock trava") vai na mesma direção.
- **Onde mexe:** `native_dock/mod.rs` e `launcher/mod.rs` (um `catch_unwind` em volta do corpo da wndproc, com log e recriação do dock); avaliar `panic = "unwind"` no release (custa alguns KB no exe); um `log::debug` (ou reenfileirar na `QUEUE`) quando `with_docks` não consegue o empréstimo; um aviso no log quando a espera de 5 s estoura.
- **Risco:** médio. Recuperar de um panic deixa estado pela metade; o mais seguro é recriar a janela do dock do zero, como na troca de monitor.
- **Como verificar:** um command só do build `e2e` que provoca um panic na pintura; o e2e confere que o dock volta e que o log tem o panic.

### M5. Contraste do texto branco sobre o cobalto no escuro

- **O que é:** botões principais com texto `#fff` sobre `var(--accent)`: `src/panel/Panel.css:283` (Atualizar, no Sobre), `src/tour/tour.css:77`, `src/widgets/pomodoro/pomodoro.css:24` e `src/widgets/water/water.css:31`. No modo escuro o accent é `#7c9cff` (`src/themes/tokens.css:97` e `:127`): dá cerca de 2,6:1, abaixo dos 4,5:1. O `contrast.test.ts` não cobre esse par.
- **Por que vale:** é o botão que mais importa em cada uma dessas telas, e o `DESIGN.md` promete AA.
- **Onde mexe:** um token `--on-accent` (escuro: o `ink` do grafite, `#1d1e22`) em `tokens.css`, usado nos quatro lugares; o mesmo `color-mix(... 86%, #000)` do hover, repetido nos quatro, vira um token; um caso novo em `src/themes/contrast.test.ts`.
- **Risco:** baixo. Muda a cor de quatro botões no escuro.
- **Como verificar:** o teste de contraste falha com `#fff` (sabotagem) e passa com o token; foto dos quatro botões nos dois modos.

### M6. Alto contraste: foco visível no front e cores no dock nativo

- **O que é:** no front, `forced-colors` só aparece em sete telas; faltam `Panel.css`, `parts/parts.css`, `ChoiceCards.css`, `DockItems.css` e `StyleGallery.css`. Pior: `parts/parts.css:229-233` tira o `outline` do campo em foco e sinaliza só com borda e sombra, que somem no alto contraste (o mesmo em `AppMenuPanel.css:77`). No Rust, só o lançador consulta `SPI_GETHIGHCONTRAST` (`launcher/mod.rs:944`); o dock e as skins ignoram o alto contraste, e o `WM_SETTINGCHANGE` do dock (`mod.rs:3495`) não refaz as cores.
- **Por que vale:** no alto contraste, hoje, quem usa teclado não vê onde está o foco em Configurações.
- **Onde mexe:** um bloco `@media (forced-colors: active)` em cada CSS citado, com `outline: 2px solid CanvasText` no foco; no dock, um `Look` de alto contraste (cores do sistema, sem skin nem vidro) escolhido quando o Windows está em alto contraste.
- **Risco:** baixo no front; baixo a médio no dock (um estilo a mais em `look.rs`).
- **Como verificar:** testes com `matchMedia` falso nos componentes; ao vivo, o tema "Deserto" do Windows com Configurações e o dock, em fotos.

### M7. Leitor de tela nas escolhas e no "Manter este tema?"

- **O que é:**
  - `ChoiceCards` (`src/panel/parts/ChoiceCards.tsx:80`, `:102`) é uma escolha única montada como `role="group"` com botões `aria-pressed`; o padrão certo é `radiogroup` com `role="radio"` e `aria-checked`. Na grade, as setas para cima e para baixo andam um cartão, não uma linha.
  - Em `ThemeGallery.tsx`, o "Manter este tema?" aparece sem região viva e sem levar o foco ao Manter, e a contagem de 15 s é `aria-hidden` (`:126`): o tema volta sozinho sem o leitor de tela ter dito nada.
  - Regiões vivas montadas já com o texto, que muitos leitores não anunciam: `AboutPanel.tsx:88`, `PanelFooter.tsx:31`, `AppsPanel.tsx:251`, `WidgetCatalog.tsx:181`, `CalendarPanel.tsx:138`. O `NotePanel.tsx:251` já faz certo (contêiner fixo, só o texto muda).
- **Por que vale:** são as telas que todo usuário passa (temas, estilos, catálogo), e a reversão silenciosa é um problema real para quem não vê a tela.
- **Onde mexe:** os arquivos citados e os testes deles.
- **Risco:** baixo. Trocar o papel muda o que os testes buscam (`getByRole("radio")`).
- **Como verificar:** testes com `getByRole("radiogroup")` e o anúncio do "Manter" (texto na região viva ao aparecer e 5 s antes de voltar); ao vivo com o Narrador, quando a rodada M11 chegar lá.

### M8. Painel e Configurações mais leves no WebView2

- **O que é:**
  - `PanelContent.tsx:83` chama `useNow(SECOND_MS)` para todo widget: notas, tarefas, clima e calendário redesenham a cada segundo sem precisar. O `ThemeGallery` redesenha todas as miniaturas a cada segundo durante a contagem.
  - A raiz de `Panel.tsx` assina configurações, widgets, apps, apps abertos e espaços: cada `open-apps-changed` redesenha a árvore inteira, e nenhum dos componentes grandes usa `memo`.
  - `widgets/registry.ts` importa 48 módulos de uma vez, sem `lazy()`: o painel de um relógio carrega o código de todos os widgets.
  - Os ícones chegam como data URL base64 (`commands/apps.rs:347`) e ficam num `Map` sem teto (`src/core/apps.ts:231`); trocar de pacote multiplica as entradas.
- **Por que vale:** o painel é a parte cara (o WebView2 volta enquanto ele está aberto); menos trabalho e menos memória enquanto ele vive.
- **Onde mexe:** o `useNow` só nos widgets que mostram hora; `memo` nas listas; `lazy()` por widget no `registry.ts`; um teto LRU (ou `convertFileSrc`/blob URL) no cache de ícones.
- **Risco:** baixo. O `lazy()` pede um `Suspense` com um estado de carregando que não pisque.
- **Como verificar:** o Profiler do React com o painel de uma nota aberto por 10 s (zero renders por segundo); memória do processo de renderização do WebView2 antes e depois, com Configurações aberta na galeria de ícones.

### M9. Documentos desatualizados

- **O que é:**
  - [`limitacoes.md`](../limitacoes.md) ainda diz que os widgets são só Relógio, Progresso e Nota, que não há pacote embutido, que não há prévia de janela e que arquivos e janelas abertas no lançador ficam para depois; chama o app de DockMax e não fala de temas, atualização automática, idioma, conta Google, nem do Clima e do Claude que usam a rede.
  - [`roadmap.md`](../roadmap.md): F "em andamento", G "depois da F", "nove estilos", o backlog com Calendário, Atividade, Música, Água, Claude, Clima e área de transferência desmarcados, o lançador descrito até o L3.
  - [`roteiro-de-verificacao.md`](../roteiro-de-verificacao.md), l. 32: tirou "Ícones dos apps" porque não havia pacote embutido; hoje há 12.
  - Status das specs [0018](../specs/0018-arrastar-entre-dock-e-pastas.md) e [0019](../specs/0019-fase-g-extras.md) ("em andamento"), a nota do Canarinho na [0004](../specs/0004-paridade-primodock.md) (l. 156) e as "Notas para o DockMax" da [referência](../reference/primodock/interface.md), que listam como faltando coisas já feitas.
- **Por que vale:** a `limitacoes.md` é o que quem instala lê; hoje ela subestima o app e omite o que usa a rede (o que importa para a [privacidade](../privacidade.md)).
- **Onde mexe:** só `docs/`.
- **Risco:** nenhum.
- **Como verificar:** releitura; um `grep -n "DockMax" docs/limitacoes.md docs/roadmap.md` só acha menções históricas.

### M10. Testes que faltam no front

- **O que é:** sem nenhum teste: `useSettingsEditor.ts` (o núcleo que grava as configurações), `src/i18n/index.ts` (o "Sistema" e a volta ao português), `core/google.ts`, `PanelContent.tsx`, `ChoiceThumbs.tsx`, `OptionGroup.tsx`, `RangeField.tsx`, os widgets de Atividade e Rede. `en.ts` não é importado por teste nenhum: nenhuma tela é desenhada em inglês. Só cobertos de passagem: `core/openApps.ts`, `workspaces.ts`, `weather.ts`, `tray.ts`, `launcher.ts`, `iconPacks.ts`, `update.ts` e o `TourApp`.
- **Por que vale:** `useSettingsEditor` e o i18n tocam todas as telas; um erro ali aparece em tudo.
- **Onde mexe:** arquivos `.test.ts(x)` ao lado de cada um; um teste que desenha Configurações e o tour em inglês e confere que nenhum texto em português sobrou (um texto fixo esquecido).
- **Risco:** baixo.
- **Como verificar:** cada teste novo pega a sabotagem do que ele guarda (por exemplo, `useSettingsEditor` sem o debounce).

### M11. Rodada de verificação ao vivo, por prioridade

- **O que é:** o [roteiro](../roteiro-de-verificacao.md) tem os blocos 1 a 6 quase todos em aberto, e cada spec desde a 0012 deixou a sua lista. Duas falhas da rodada de 27/10 (um programa fixado aparecendo duplicado e o dock do monitor principal voltando sobre a tela cheia) não se repetiram, mas também não tiveram causa achada (roteiro, l. 24). A atualização automática (G3) não tem verificação ao vivo registrada. Proposta de ordem: (1) tentar reproduzir as duas falhas com a sonda `spike_open_and_full_screen`; (2) a atualização de uma versão para a seguinte pelo updater; (3) uso longo (horas) com memória e CPU, junto da M1; (4) dois monitores (ligar e desligar com o app aberto); (5) Narrador no dock, no lançador e em Configurações; (6) o resto.
- **Por que vale:** muita coisa entrou testada só por código e por e2e simulado; as duas falhas sem causa são do tipo que faz o usuário desistir.
- **Onde mexe:** só o roteiro e, para cada falha confirmada, um PR de correção.
- **Risco:** baixo, mas cada teste ao vivo mexe na barra do Windows: combinar antes com as outras sessões e restaurar a barra no fim.
- **Como verificar:** o roteiro marcado, com data e foto em `docs/fotos/`.

### M12. Dívida técnica: arquivos grandes e código repetido

- **O que é:**
  - `native_dock/mod.rs` tem 4 110 linhas. Partes que saem com pouco risco: auto-ocultar (~1400–1510), arrastar e soltar (~1084, 1532, 2591), cartão e prévia (2081–2320), menus (2859, 3329) e a API entre threads (3546–3870).
  - O dock e o lançador repetem o mesmo esqueleto de janela nativa com fila (`OnceLock` + `QUEUE` + `RefCell` + "posta e espera 5 s"; `native_dock/mod.rs:163-171`, `launcher/mod.rs:182-190`) e dois providers de UI Automation parecidos (`access.rs` de cada um).
  - No front, `SettingsPanel.tsx` tem dois `return` completos (Configurações, 146–416; painel, 418–668) com cerca de 19 controles repetidos; o mesmo hook (`listen` + `invoke` + estado) está copiado em nove arquivos de `src/core/` (`apps.ts:304`, `settings.ts:454`, `widgets.ts:746`…), e só dois tratam a falha do `listen` (`calendar.ts:123`, `update.ts:64`); `core/widgets.ts` mistura os tipos de todos os widgets com o IPC.
- **Por que vale:** cada PR novo no dock passa por um arquivo de 4 mil linhas; as cópias do hook são onde o próximo erro de `unlisten` vai nascer.
- **Onde mexe:** módulos novos em `native_dock/` (`autohide.rs`, `drag.rs`, `card.rs`, `api.rs`), um `windows/native_window.rs` comum, um `useTauriSnapshot(evento, command)` em `src/core/`, os campos de `SettingsPanel` descritos uma vez com dois layouts.
- **Risco:** médio. Refatoração sem mudar comportamento; fazer um módulo por PR, só movendo código, com os testes passando a cada passo.
- **Como verificar:** CI verde a cada PR; `wc -l` dos arquivos; nenhum teste muda além dos `use`.

### M13. Tokens do `DESIGN.md` que não viraram CSS

- **O que é:** os raios `inset 6px`, `control 4px` e `meter 3px` do `DESIGN.md` não existem em `tokens.css`, e há raios literais (`AppMenuPanel.css:54`, `DockItems.css:150`, `Look.css:97`, `Workspaces.css:24` com `999px` em vez de `--r-pill`). A escala de texto (12,5 / 12 / 11 px) não virou `--fs-*`: são 123 `font-size` literais. O brilho de vidro está copiado em três CSS (`Panel.css:207-219`, `DockItems.css:102-122`, `SettingsApp.css:125`) e o fundo das miniaturas em dois (`ChoiceThumbs.css:155`, `StyleGallery.css:22`). Os dois blocos do tema escuro em `tokens.css` (`:95` e `:127`) são mantidos iguais à mão.
- **Por que vale:** os temas novos (e os widgets no tom do tema, já adiados) vão precisar trocar essas coisas por token; hoje cada valor literal é um lugar a esquecer.
- **Onde mexe:** `src/themes/tokens.css` e os CSS citados; um teste que compara os dois blocos escuros.
- **Risco:** baixo. Mudança visual nula se os valores forem os mesmos.
- **Como verificar:** fotos antes e depois iguais; o teste dos blocos escuros falha se um deles mudar sozinho.

### M14. Teclado no dock nativo

- **O que é:** o dock responde `IsKeyboardFocusable` falso (`native_dock/access.rs:285`), não trata `WM_KEYDOWN` e não emite eventos de UI Automation (nenhum `UiaRaise*` em `native_dock`): o Narrador não fica sabendo quando um item entra, sai ou ganha selo. Proposta: um atalho (por exemplo, Win+Alt+D, configurável como o do lançador) que leva o foco ao dock; setas andam pelos itens, Enter abre, Shift+F10 abre o menu, Esc devolve o foco ao programa de antes; e os eventos de estrutura e de nome.
- **Por que vale:** sem a barra do Windows, quem não usa mouse não tem como chegar aos programas fixados pelo dock.
- **Onde mexe:** `native_dock/mod.rs`, `native_dock/access.rs`, o registro de atalhos do lançador; `i18n.rs`; Configurações › Geral.
- **Risco:** médio. O dock hoje nunca é ativado de propósito (o programa em uso fica na frente); ativá-lo só pelo atalho e devolver o foco com cuidado.
- **Como verificar:** testes das regras de navegação; ao vivo com o Narrador, no bloco 5 da M11.

### M15. Lançador na escala nova ao trocar de monitor

- **O que é:** `launcher/mod.rs:2858` ignora `WM_DPICHANGED` porque o lançador se posiciona a cada abertura. Se ele estiver aberto quando a escala ou o monitor mudam, fica na escala antiga.
- **Por que vale:** caso raro, mas fácil de fechar.
- **Onde mexe:** `launcher/mod.rs` (refazer o layout com o retângulo sugerido).
- **Risco:** baixo.
- **Como verificar:** ao vivo, mudar a escala de 125 % para 150 % com o lançador aberto.

## Funcionalidades

Entram depois dos quatro itens adiados da spec 0020. A ordem pesa o valor no uso diário, o custo de memória e o que o autor já decidiu: o mascote, o IconStudio, a licença e o widget de Bateria ficaram de fora por decisão dele e não aparecem aqui.

### F1. Economia de bateria

- **O que é:** um modo, ligado sozinho na bateria (ou sempre), que para o letreiro, o pulo ao abrir, as animações das miniaturas e as consultas que não são urgentes (status a cada 30 s, clima e atualização mais espaçados). O PrimoDock tem a mesma chave em Geral › App ([spec 0004](../specs/0004-paridade-primodock.md) §3.7b).
- **Por que vale:** casa com a prioridade de poupar recursos e é barato depois da M3.
- **Onde mexe:** `domain/settings.rs` e `src/core/settings.ts` (`powerSaver`: desligado, na bateria, sempre); `system_status.rs` (o estado da energia, que já lê); os intervalos de `native_dock`, `media.rs`, `weather.rs`, `windows/update.rs`; Configurações › Geral; `i18n`.
- **Risco:** baixo.
- **Como verificar:** testes das regras (na bateria → intervalos longos); ao vivo, tirar o carregador e ver o letreiro parar.

### F2. Espaçadores no dock

- **O que é:** espaços vazios pequenos e grandes entre os itens, por espaço de trabalho, como no PrimoDock; "Adicionar espaçador pequeno/grande" no menu do dock e remover pelo botão direito. Adiado na [0013](../specs/0013-fase-c-espacos-de-trabalho.md) §5 e na [0017](../specs/0017-fase-f-widgets.md) §5.
- **Por que vale:** organizar o dock em grupos é pedido comum em docks e custa quase nada de memória.
- **Onde mexe:** `domain/dock_items.rs` e `dock_layout.rs` (um item `Spacer`), `persistence/workspaces_repo.rs` (migração), `windows/menu/`, `DockItems.tsx` (a lista em Configurações), `i18n`.
- **Risco:** baixo; precisa de uma migração do banco.
- **Como verificar:** testes do layout com espaçadores nas quatro bordas; arrastar um espaçador para reordenar.

### F3. Pilhas de widgets

- **O que é:** "Empilhar com" no menu de um widget junta dois ou mais num só ladrilho; a roda com Ctrl (ou a roda lateral) troca o widget de cima, com pontinhos indicando a posição. Adiado na 0013 e na 0017.
- **Por que vale:** com 17 tipos de widget, o dock enche; a pilha mostra mais coisas no mesmo espaço.
- **Onde mexe:** `domain/widgets/mod.rs`, `dock_layout.rs`, `native_dock/faces.rs`, `persistence/widgets_repo.rs` (a pilha a que o widget pertence), `windows/menu/`.
- **Risco:** médio. Os widgets que leem dados (mídia, atividade) devem ler só quando estão em cima, para não gastar à toa.
- **Como verificar:** testes da pilha (empilhar, desempilhar, girar, apagar o de cima); ao vivo, a roda em cima da pilha.

### F4. Ícone por app (B4b)

- **O que é:** "Ícone ›" no menu de um programa: Original, um dos pacotes instalados, ou "Escolher imagem…" (PNG ou ICO). Recorte B4b da [spec 0007](../specs/0007-fase-b4-icones.md), nunca começado.
- **Por que vale:** os pacotes não cobrem todo programa (os jogos ficam na reserva, Q4 da 0020); deixa o usuário fechar o buraco sem esperar um pacote novo.
- **Onde mexe:** `domain/icon_pack.rs`, `persistence/` (uma tabela de escolhas por programa, ou um campo do item), `windows/menu/`, `native_dock/mod.rs` (a chave do cache), `commands/apps.rs`, uma imagem copiada para a pasta de dados.
- **Risco:** médio. A imagem do usuário precisa ser lida com limite de tamanho e decodificada fora da thread do dock.
- **Como verificar:** testes da escolha (pacote vence o original, imagem vence o pacote); a imagem sobrevive a trocar de pacote global.

### F5. Downloads, Lixeira e pastas no dock

- **O que é:** fixar uma pasta no dock e, ao clicar, abrir uma grade dos arquivos mais recentes dela (como as pilhas do macOS), com "Abrir no Explorador" no fim. Downloads e Lixeira como itens prontos (a Lixeira com ícone cheio ou vazio e "Esvaziar" no menu, com confirmação). O PrimoDock tem Downloads e Lixeira; o widget Arquivos já está declarado e não pode ser criado (ADR 0008).
- **Por que vale:** Downloads é a pasta mais aberta do dia; e a Lixeira some junto com a barra do Windows.
- **Onde mexe:** um tipo de item novo em `domain/dock_items.rs`; um cartão em Direct2D como o da agenda (`windows/agenda.rs`) ou um painel WebView2; `windows/shell_target.rs`; a Lixeira por `SHQueryRecycleBin`/`SHEmptyRecycleBin`.
- **Risco:** médio. Pastas grandes: ler só os N mais recentes, numa thread, sem guardar a lista depois de fechar.
- **Como verificar:** testes da ordenação e do limite; ao vivo, baixar um arquivo e vê-lo no topo.

### F6. Busca dentro de Configurações

- **O que é:** um campo "Buscar nos ajustes" no topo da barra lateral, como no PrimoDock, que filtra as seções e leva à opção destacada. O lançador já acha as seções (spec 0012 L2); falta achar a opção, dentro da janela.
- **Por que vale:** Configurações cresceu muito (temas, estilos, pacotes, widgets, contas, lançador).
- **Onde mexe:** `src/settings/SettingsApp.tsx`, um índice das opções gerado das chaves do i18n, nos dois idiomas.
- **Risco:** baixo.
- **Como verificar:** testes do índice (buscar "cantos" leva a Aparência › Personalizar); teclado e leitor de tela no campo.

### F7. Pequenos do PrimoDock

- **O que é:** três itens pequenos que a [referência](../reference/primodock/interface.md) mostra e nenhuma spec decidiu: "Procurar…" para fixar um `.exe` qualquer (hoje só pelo catálogo ou arrastando), "Nova janela" no menu de um programa aberto (hoje "nova instância" está no botão do meio) e "Duplicar" um espaço de trabalho.
- **Por que vale:** cada um fecha um caminho que hoje exige arrastar ou refazer à mão.
- **Onde mexe:** `windows/menu/`, `commands/apps.rs` (o diálogo de abrir arquivo), `persistence/workspaces_repo.rs`, `WorkspacesPanel.tsx`.
- **Risco:** baixo.
- **Como verificar:** um teste por item; ao vivo, fixar um `.exe` de uma pasta portátil.

### F8. Exportar e importar as configurações

- **O que é:** em Sobre (ou Geral), "Exportar…" grava um arquivo `.dockyard` com o `settings.json`, os espaços, os itens e os widgets (sem tokens das contas); "Importar…" lê, mostra o que vai mudar e aplica, com uma cópia de segurança antes. Hoje a [limitação](../limitacoes.md) manda copiar a pasta de dados com o app fechado.
- **Por que vale:** trocar de computador ou reinstalar o Windows sem perder o dock montado; e uma volta segura antes de experimentar muito.
- **Onde mexe:** `persistence/database.rs` e `settings_file.rs` (um formato versionado), `commands/`, `AboutPanel.tsx`.
- **Risco:** médio. Importar de uma versão antiga passa pelas migrações; nunca levar segredos (o Credential Manager fica de fora).
- **Como verificar:** testes de ida e volta (exportar e importar dá o mesmo banco); importar um arquivo de versão antiga; um arquivo corrompido não muda nada.

### F9. Widgets que faltam do catálogo

- **O que é:** os que a 0017 §5 deixou para depois e não foram recusados pelo autor: Layout do teclado (troca com um clique), Relógio mundial (vários fusos num cartão largo), Capturas de tela (as últimas da pasta, arrastáveis) e Arquivos (já declarado). Codex e OpenCode só se o autor usar.
- **Por que vale:** completa o catálogo do PrimoDock com o que tem uso diário.
- **Onde mexe:** um par `domain/widgets/<tipo>.rs` + `src/widgets/<tipo>/` por widget, como os da fase F.
- **Risco:** baixo por widget; cada um mede a memória com o widget no dock.
- **Como verificar:** os testes de contrato dos widgets (`contract_tests.rs`, `registry.test.tsx`) e um e2e por widget.

### F10. Prévias: Espiar e cartão pelo teclado

- **O que é:** passar o mouse numa miniatura do cartão deixa as outras janelas transparentes e mostra só aquela (o "Espiar" do Windows); e o cartão navegável pelas setas, com Enter e Delete. Fora da fase na [0015](../specs/0015-fase-e-previas.md) §5.
- **Por que vale:** é o que mais falta a quem vem da barra do Windows nas prévias.
- **Onde mexe:** `windows/previews.rs` (`DwmpActivateLivePreview` não é documentada; avaliar antes um spike), `domain/previews.rs`; o teclado depende da M14.
- **Risco:** médio a alto no Espiar (API não documentada); baixo no teclado.
- **Como verificar:** spike com resultado escrito antes de prometer; testes da navegação.

### F11. Selos na bandeja e selos sem número

- **O que é:** o ponto do Teams e de outros que marcam "tem novidade" sem número, e o selo nos ícones da bandeja no dock. Pendentes na [0016](../specs/0016-selos-de-notificacao.md) §7.
- **Por que vale:** os programas de conversa mais usados no trabalho não mostram número.
- **Onde mexe:** `windows/badges.rs`, `tray_host.rs`, `native_dock/paint.rs`.
- **Risco:** médio. Depende do que a UI Automation da barra expõe; um spike primeiro.
- **Como verificar:** ao vivo com o Teams; testes da leitura com árvores gravadas.

### F12. Dock vertical completo para widgets

- **O que é:** com o dock na esquerda ou na direita, os widgets largos viram altos e o texto gira (como no PrimoDock); hoje o tile largo na vertical está como "futuro" na 0017 §3.
- **Por que vale:** fecha a posição lateral, que hoje fica pela metade para quem usa widgets.
- **Onde mexe:** `domain/dock_layout.rs`, `native_dock/faces.rs`, `paint.rs`.
- **Risco:** médio: cada face de widget precisa de um layout alto.
- **Como verificar:** testes do layout nas quatro bordas; fotos de cada widget nas laterais.

### F13. Itens e espaço por monitor

- **O que é:** hoje todos os docks mostram os mesmos itens e o mesmo conjunto de espaços. Proposta: cada monitor escolhe o seu espaço ativo de forma independente e, opcionalmente, os seus itens. É a ideia 5 da [referência](../reference/primodock/comportamentos.md).
- **Por que vale:** quem usa dois monitores costuma ter um para cada coisa.
- **Onde mexe:** o modelo de espaços (`domain/`, `persistence/workspaces_repo.rs`, `window_spaces.rs`), o dock por monitor, Configurações › Espaços.
- **Risco:** alto: mexe no modelo de dados e no que acontece com as janelas ao trocar de monitor (pergunta em aberto da 0013 §6).
- **Como verificar:** spec própria; testes do modelo; ao vivo com dois monitores.

### F14. Extensões do lançador

- **O que é:** o §9 da [spec 0012](../specs/0012-lancador.md): palavras-chave que chamam um script local e mostram o resultado, como as extensões do Ulauncher.
- **Por que vale:** é o que torna o lançador uma ferramenta, não só uma busca.
- **Onde mexe:** `windows/launcher/`, `domain/launcher.rs`, Configurações › Lançador.
- **Risco:** alto: rodar código de terceiros. Exige um modelo de permissão (o usuário aprova cada extensão, processo separado, tempo limite).
- **Como verificar:** spec e ADR antes; testes do isolamento.

### F15. Temas sazonais

- **O que é:** os enfeites de Halloween e de Natal do PrimoDock (luzes, guirlanda, abóboras) sobre qualquer tema, saídos "por enquanto" na 0004 (l. 156). Com as skins de 9 fatias e a galeria de Temas, viram uma camada por cima da barra.
- **Por que vale:** charme de baixo custo, agora que a infraestrutura existe. Fica por último porque o autor tirou da fila; precisa de uma decisão dele.
- **Onde mexe:** `src-tauri/skins/`, `native_dock/paint.rs` (uma camada de enfeite), `src/core/themes.ts`.
- **Risco:** baixo; animação parada na Economia de bateria (F1).
- **Como verificar:** fotos; memória com o enfeite animado.

## O que ficou de fora

- Os quatro itens adiados da spec 0020 (ver "Como ler").
- O mascote, o IconStudio, a licença e o widget de Bateria: o autor decidiu deixar de fora (0019 §1, 0004 §3.4, 0017 §1).
- O Linux: o [estudo 0001](0001-linux.md) recomenda não portar agora.
- Integrações novas de conta (Notion, iCal, conta Microsoft), já registradas como "para quando fizer falta" na 0017 §6c.
