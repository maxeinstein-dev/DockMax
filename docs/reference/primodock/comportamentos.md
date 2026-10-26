# Comportamentos do PrimoDock, pelo registro de execução

Fonte: `%APPDATA%\PrimoDock\primodock.log` na máquina do autor, de 2026-10-20 a 2026-10-25 (1188 linhas, 44 tipos de mensagem). Cada linha é `HH:mm:ss.fff mensagem`, sem data nem nível; o arquivo é reescrito a cada abertura. As linhas sobre a licença e o identificador do dispositivo foram ignoradas de propósito.

Os números entre parênteses são quantas vezes a mensagem apareceu no período.

## Barra de tarefas do Windows

- Ao ocultar, liga o ocultar automático da barra **e** esconde as janelas dela: `taskbar auto-hide -> True` (157) e `taskbar windows -> hidden` (195). As duas coisas se repetem várias vezes por segundo logo depois de abrir e depois de cada mudança de monitor: o Explorer devolve a barra e o PrimoDock a esconde de novo.
- Guarda se a barra já estava em ocultar automático antes (`taskbarAutoHideBefore` no `settings.json`) para devolvê-la como estava.
- Tem um **vigia** fora do processo do dock: `watchdog: taskbar restored` (3) aparece quando o dock termina sem devolver a barra.
- Detecta que o processo do dock **travou**: `stall: the Dock has not answered for 5 s` e, depois, `stall: the Dock answers again` (5 de cada). O vigia observa o dock e não só a saída dele.
- Ao sair normalmente: `taskbar auto-hide -> False` (6), `taskbar windows -> shown` (4).

O DockMax já faz o mesmo (ADR 0009: estado em arquivo, processo guarda fora da árvore, recuperação na abertura). Fica como ideia o aviso de **dock travado**, que o DockMax não tem.

## Barra de aplicativos (a faixa reservada)

- `app bar: registered -> True` (38) e `app bar: asked 71px at Bottom of \\.\DISPLAY2 0,0,2560,1080, granted 0,1009,2560,1080` (62): pede a faixa por monitor, com o retângulo do monitor e o concedido.
- **Espessura da faixa**: 71 px em baixo num monitor a 100 % com ícones de 52 px e `bottomSpace` 2; 89 px no monitor a 125 % (71 × 1,25). Em cima e à direita, 70 px. Ou seja, a faixa tem em torno de 71 px lógicos para ícones de 52.
- `app bar: the reserved strip was lost (Explorer restarted?), taking it again` (3): quando o Explorer reinicia e a faixa some, pede de novo.
- Também à direita (`asked 70px at Right`) e em cima (`at Top`): as quatro bordas funcionam com a faixa reservada.
- `display scale is now 125%` (6): acompanha a mudança de escala do monitor.

## Vários monitores

- `a Dock on every monitor` (8) e `docks: 2 (\\.\DISPLAY2, \\.\DISPLAY1)`: um dock por monitor quando `onEveryDisplay` está ligado, e 1 quando o segundo monitor sai.
- Cada monitor tem seu espaço de trabalho: `switch \\.\DISPLAY2 to Personal` (6), `to Focus` (5), `to Work` (4). A troca é por monitor, não global (`displays` no `workspaces.json`).

## Apps em tela cheia

- `full screen app in front: hiding the Dock` e `full screen app gone: the Dock comes back` (190 de cada): o dock some quando um app em tela cheia (vídeo, jogo, apresentação) vai para a frente, e volta quando ele sai. Independe de `autoHide`.

O DockMax ainda não faz isso: entra na fase D (janelas e apps em execução).

## Bandeja do sistema

- `tray: hosting the notification area` (8): hospeda a área de notificação (a biblioteca ManagedShell, ver [instalacao.md](instalacao.md)).
- `tray: panel opened with 7 icons` (34): o painel da bandeja com a contagem de ícones.
- `tray: another app came to the front, closing the panel` (19): o painel fecha quando outro app vai para a frente.
- `tray: path:<exe>#<n> in the Dock` e `back in the panel`: fixar e desafixar um ícone da bandeja no próprio dock, pela chave `path:` (ver [settings-e-dados.md](settings-e-dados.md)).
- `tray: icons given back to Explorer` (4): ao sair, devolve os ícones ao Explorer.
- `quick settings: focus handed over -> True` (9) / `False` (4): ao abrir as configurações rápidas do Windows (bateria, rede e volume), tenta passar o foco a elas; às vezes o Windows recusa.

## Visual das janelas

- `glass acrylic tint 8C1E1E22 -> True` (32): o fundo de vidro acrílico do Windows (`SetWindowCompositionAttribute`), com a cor em ARGB. Tons vistos: `8C1E1E22` (55 % de `#1E1E22`, o dock escuro), `8C0B1020` (55 % de `#0B1020`, o tema Meia-noite) e `D81C1C21` (85 % de `#1C1C21`, o painel da bandeja, mais opaco).
- `round corners -> 0x00000000` (73): define a preferência de cantos do DWM a cada janela que abre (o valor 0 é o padrão do Windows 11).

## Widgets, galeria e atualização

- `widget panel: Activity` (4), `widget panel: Network` (3): o painel de um widget abre com o nome dele.
- `icon gallery: installed pixel-system 1.2.0`: instalação de um pacote pela galeria.
- `app catalog: 168 apps of 197 entries (0 unreadable)` (11): o catálogo de programas tem 197 entradas e 168 apps depois do filtro (o DockMax lista as mesmas 197 da pasta Aplicativos do shell; o PrimoDock tira 29, provavelmente desinstaladores, manuais e atalhos que não são programas).
- `update: up to date (0.7.0-beta) at https://dl.criador.studio/windows/win-x64` (11): procura atualização a cada abertura (Velopack). Sem rede, `check failed` e a galeria `catalog not read`, e segue funcionando.

## Ideias para o DockMax, em ordem de valor

1. Esconder o dock com app em **tela cheia** na frente (fase D).
2. **Filtrar o catálogo** de apps: tirar desinstaladores, manuais e atalhos de documentos (de 197 para cerca de 168).
3. Avisar e se recuperar quando o dock **trava** por alguns segundos.
4. Pedir a faixa de novo quando o **Explorer reinicia** (conferir se o DockMax já cobre: ele reage a `TaskbarCreated`).
5. **Espaço de trabalho por monitor**, quando houver dock por monitor.
