# Estudo 0001 — DockMax no Linux

Recorte G5 da [spec 0019](../specs/0019-fase-g-extras.md): o que seria preciso para o DockMax funcionar no Linux. É só estudo: nada do app mudou. Fontes consultadas em 2026-10-29; versões de crates e de compositores são as daquele dia.

## Resumo

- Mais da metade do Rust é Windows: `src-tauri/src/windows/` tem ~30,5 mil linhas, contra ~15,3 mil do domínio, ~5,1 mil da persistência e ~4,9 mil dos comandos. O dock em si (`native_dock`, ~8,9 mil linhas) é uma janela Win32 desenhada com Direct2D, e nada dele serve no Linux.
- O domínio, a persistência, o contrato de widgets e o front React são portáveis, mas o domínio fala a língua do Windows (AUMID, `.exe`/`.lnk`, códigos de tecla virtuais) e não há traits que isolem o sistema: a lista de "ports" da [arquitetura](../architecture.md) (§7) nunca virou código.
- No Linux, "um dock" não é uma coisa só. No **Wayland** cada compositor decide o que um app de fora pode fazer: KDE Plasma e os baseados em wlroots (Sway, Hyprland, Labwc…) deixam um dock se posicionar e, com protocolos diferentes, ver as janelas dos outros; o **GNOME (Mutter) não deixa nenhuma das duas coisas**. No **X11** tudo é possível (EWMH), mas o X11 está saindo: o GNOME 50 já é só Wayland.
- O **Zorin OS 18**, a inspiração do autor, é GNOME 46 em Wayland por padrão: é justamente o caso mais difícil. Lá um dock só existe como extensão do GNOME Shell (é o que o próprio Zorin Taskbar é) ou na sessão X11, que o Zorin 18 ainda traz.
- **Recomendação** (detalhes no §5): não portar agora. Se o Linux virar meta, o caminho é (a) KDE Plasma + wlroots com layer-shell e o dock desenhado em Rust puro (tiny-skia), precedido de um passo que vale mesmo sem Linux: separar o que é Windows atrás de traits. GNOME/Zorin fica de fora, ou vira um projeto à parte (extensão do Shell).

## 1. O que é Windows no código

### 1.1 Dependências

Em [`src-tauri/Cargo.toml`](../../src-tauri/Cargo.toml), tudo que é do Windows já está em `[target.'cfg(windows)'.dependencies]`: o crate `windows` 0.62 com ~40 features (Direct2D, DirectWrite, DWM, WinHTTP, Credentials, Shell, UI Automation, Media Control, Notifications…), `windows-core`, `windows-numerics` e `webview2-com`. As dependências comuns (Tauri 2, rusqlite com SQLite embutido, jiff com tzdb embutida, uuid, png, base64, url, serde) já são multiplataforma.

### 1.2 Módulos de `src-tauri/src/windows/`

Quase todo módulo é `#[cfg(windows)]` em [`windows/mod.rs`](../../src-tauri/src/windows/mod.rs). Os que não são (`apps`, `autostart`, `dock`, `jumplist`, `memory`, `panel`, `panel_window`, `settings_window`) têm trechos `#[cfg(not(windows))]` que devolvem vazio, e `commands/` tem ~20 desses trechos. Ninguém compilou isso fora do Windows: o CI ([`ci.yml`](../../.github/workflows/ci.yml)) roda `clippy` e `test` só em `windows-latest`.

| Área | Módulos (linhas) | O que usa do Windows |
|---|---|---|
| Desenho do dock | `native_dock/` (8 949), `dock.rs` (660) | Janela em camadas Win32, Direct2D, DirectWrite, WIC; UI Automation em `access.rs` |
| Lançador e menus próprios | `launcher/` (3 099), `menu/` (1 281), `popup_menu.rs` | Janelas Win32 + Direct2D; `RegisterHotKey` para o Alt+Espaço |
| Reserva da borda e barra | `appbar.rs` (519), `taskbar.rs` (885), `fullscreen.rs`, `monitors.rs` | `SHAppBarMessage`, esconder/devolver a barra do Windows, monitores e tela cheia |
| Janelas abertas | `openwin.rs` (440), `open_apps.rs` (786), `window_spaces.rs` (490), `profiles.rs` | `EnumWindows`, `SetWinEventHook`, AUMID das janelas, `SetForegroundWindow`, esconder janelas para os espaços de trabalho, perfis do Chrome |
| Prévias | `previews.rs` (1 088) | Miniaturas ao vivo do DWM (`DwmRegisterThumbnail`) |
| Bandeja | `tray_host.rs` (1 580) | Hospeda a área de notificação (`Shell_NotifyIcon` dos outros programas) |
| Selos | `badges.rs` (405) | Lê os selos que os programas põem na barra do Windows |
| Programas instalados e menu | `apps.rs` (614), `jumplist.rs` (794), `open_with.rs`, `shell_target.rs`, `drop_target.rs` | `shell:AppsFolder`, ícones do shell, Jump Lists (formato não documentado), OLE drag-and-drop, Explorer |
| Painéis do sistema | `flyout.rs`, `system_status.rs`, `system_activity.rs` | Atalhos Win+A/Win+N, bateria, rede, volume (Core Audio), CPU/memória/rede |
| Mídia | `media.rs` (398) | GSMTC (`GlobalSystemMediaTransportControlsSessionManager`) |
| Rede | `http.rs` (200) | WinHTTP: TLS, certificados e proxy do sistema, sem biblioteca de rede no binário |
| Segredos e contas | `credentials.rs`, `google.rs`, `claude_usage.rs` | Credential Manager; OAuth com loopback (portável, exceto o cofre e o http) |
| Avisos | `toast.rs` | Toasts do Windows com AppUserModelID registrado |
| Início automático | `autostart.rs` | Chave `Run` do registro |
| WebView | `memory.rs`, `recovery.rs`, `native.rs`, `panel*.rs`, `settings_window.rs` | Meta de memória do WebView2, volta quando o processo do navegador cai, esconder do Alt+Tab |
| Widgets com timer | `pomodoro.rs`, `water.rs`, `weather.rs`, `calendar.rs`, `agenda.rs` | Pouco de Windows: threads e http; `agenda.rs` (837) é um cartão desenhado com Direct2D |

### 1.3 O que é portável

- **Domínio** (`src-tauri/src/domain/`, ~15,3 mil linhas): regras puras e testadas (layout do dock, itens, widgets, espaços, lançador, calculadora, agenda, ciclo, clima, selos, prévias). Portável, com ressalvas: `open_apps.rs` casa janelas por AUMID, `apps.rs` aceita `.exe`/`.lnk`, `hotkey.rs` fala em VK e reserva os atalhos do Windows, `badges.rs` e `browser_profile.rs` supõem o jeito do Windows. No Linux a identidade de um programa é o `app_id` do Wayland / `WM_CLASS` do X11 casado com o nome do arquivo `.desktop`.
- **Persistência** (`persistence/`, ~5,1 mil linhas): SQLite embutido e `settings.json`. Portável; os dois únicos `cfg(windows)` são testes. A pasta de dados viria do Tauri (`~/.local/share/dev.maxsueleinstein.dockmax`).
- **Front** (`src/`, ~33 mil linhas de TS/TSX): painéis e Configurações em React. Portável no código; os textos citam o Windows ("Iniciar com o Windows", "barra de tarefas do Windows", "Jump List") e seções inteiras de Configurações (barra de tarefas, bandeja no dock, painéis do Windows) teriam que sumir ou mudar no Linux.
- **i18n, logs, contrato de widgets**: portáveis.

## 2. O equivalente no Linux, área por área

Legenda dos ambientes: **wlroots** = Sway, Hyprland, Labwc, river, Wayfire e afins; **KDE** = Plasma 6 (KWin) em Wayland; **GNOME** = Mutter em Wayland (inclui Zorin e Ubuntu); **X11** = qualquer gerenciador de janelas em Xorg.

### 2.1 O panorama dos ambientes

- O Zorin OS 18 (out. 2025) é Ubuntu 24.04.3 com GNOME Shell 46 personalizado, suporte até 2029; o 18.1 (abr. 2026) segue no GNOME 46.7 [Z1][Z2][Z3]. A sessão padrão é Wayland, com Xorg instalado como opção (e X11 forçado com driver NVIDIA proprietário) [Z4 — fonte de fórum, não oficial].
- O Zorin Taskbar e o Dash to Panel são da mesma família: o Dash to Panel nasceu de um fork do Zorin Taskbar, que depois foi refeito sobre o Dash to Panel; a Zorin patrocina o Dash to Panel desde 2025 [Z5]. Ou seja, o dock do Zorin é uma **extensão do GNOME Shell em JavaScript**, como o Dash to Dock [D1].
- O X11 está saindo do GNOME: o Ubuntu 25.10 tirou a sessão "Ubuntu on Xorg" [Z6]; o GNOME 49 desligou a sessão X11 por padrão e o GNOME 50 (mar. 2026) é só Wayland, com XWayland para os apps [Z7]. Inferência (não confirmada): o próximo Zorin, sobre um Ubuntu com GNOME 50, não terá sessão X11.

### 2.2 Tabela de equivalentes

| Área (Windows) | X11 | Wayland: wlroots | Wayland: KDE | Wayland: GNOME |
|---|---|---|---|---|
| **Janela do dock e reserva da borda** (app bar) | `_NET_WM_WINDOW_TYPE_DOCK` + `_NET_WM_STRUT_PARTIAL` (EWMH) [X1], via `x11rb` [X2] | `zwlr_layer_shell_v1` com zona exclusiva [W1] | layer-shell (KWin 6.7 na tabela) [W1] | **Não há**: Mutter não implementa layer-shell [W1]. Só extensão do Shell |
| **Desenho** (Direct2D/DirectWrite) | Qualquer: tiny-skia 0.12 (CPU) [R1], vello 0.11/wgpu 30 (GPU) [R2], skia-safe [R3], femtovg [R4]; texto com cosmic-text 0.19 [R5]; ou GTK4 | Igual; com GTK4 há `gtk4-layer-shell` 0.8.1 [W2]; sem GTK, `smithay-client-toolkit` 0.21 tem `shell::wlr_layer` [W3] | Igual ao wlroots | — |
| **Lista de janelas, ativar, minimizar, fechar** | `_NET_CLIENT_LIST`, `_NET_ACTIVE_WINDOW`, `_NET_WM_STATE` [X1] | `wlr-foreign-toplevel-management` (Hyprland, Sway, niri, Labwc, river, Wayfire…) [W4]; `ext-foreign-toplevel-list-v1` só lista (também COSMIC) [W5] | `org_kde_plasma_window_management`: privilegiado, "clientes comuns não devem usar", um cliente por vez [W6]; o KWin libera pela chave `X-KDE-Wayland-Interfaces` no `.desktop` [W7][W8]. Se a regra mudou no Plasma 6: não confirmado | **Não há** protocolo. Docks do GNOME são extensões [D1] |
| **Prévias ao vivo** (DWM thumbnails) | XComposite + XDamage (não pesquisado a fundo) | `ext-image-copy-capture-v1` + fonte por janela de `ext-image-capture-source-v1`: Sway, Labwc, river, COSMIC; **não** Hyprland nem niri [W9][W10] | Não implementa os protocolos ext de captura [W9]; o KWin tem os seus, privilegiados (não pesquisado) | Não há |
| **Espaços de trabalho** (esconder janelas) | Áreas de trabalho EWMH (`_NET_WM_DESKTOP`) | `ext-workspace` ou IPC do compositor (não pesquisado) | Atividades/áreas do KWin (não pesquisado) | Não há |
| **Bandeja** (hospedar ícones dos outros) | StatusNotifierItem: o dock seria `StatusNotifierHost` e, se ninguém mais for, o `StatusNotifierWatcher` [T1]; crate `system-tray` 0.8 (cliente SNI + DBusMenu) [T2] | Igual (D-Bus, independe do compositor) | O Plasma já é host; dois hosts convivem [T1] | Sem bandeja nativa; a extensão AppIndicator faz o papel (não confirmado nesta pesquisa) |
| **Selos de notificação** | Sem padrão universal; `com.canonical.Unity.LauncherEntry` por D-Bus (não pesquisado) | Igual | Igual (o Plasma lê esse sinal) | Igual |
| **Mídia** (GSMTC) | MPRIS2 por D-Bus [M1]; crates `mpris` 2.1, `zbus` 6.1 [M2][M3] | Igual | Igual | Igual |
| **HTTPS** (WinHTTP) | `reqwest` + `rustls` (+ certificados do sistema); quebra a regra "nenhuma biblioteca de rede no binário" do ADR 0012 | Igual | Igual | Igual |
| **Segredos** (Credential Manager) | Secret Service API por D-Bus [S1]; `keyring` 4.2 (sobre `keyring-core`, store `zbus-secret-service-keyring-store`) [S2], ou `oo7` [S3] / `secret-service` [S4] | Igual | KWallet atende o Secret Service | GNOME Keyring |
| **Atalho global** (`RegisterHotKey`) | `global-hotkey` (base do `tauri-plugin-global-shortcut`): "Linux (X11 Only)" [K1] | Portal `GlobalShortcuts` v2 [K2], via `ashpd` [K3]: o portal do Hyprland implementa; o `xdg-desktop-portal-wlr` não [K4] | Portal implementado [K4] | Portal desde o GNOME 48 [K5]; Zorin 18 tem GNOME 46: **não tem** |
| **Início automático** (chave `Run`) | `~/.config/autostart/dockmax.desktop` (XDG Autostart) [A1]; `tauri-plugin-autostart` 2.7 [A2] | Igual | Igual | Igual; no Flatpak, portal `Background.RequestBackground` com `autostart` [A3] |
| **Programas instalados e ícones** (`shell:AppsFolder`) | Arquivos `.desktop` (Desktop Entry spec) [I1] e tema de ícones (Icon Theme spec) [I2]; crates `freedesktop-desktop-entry` 0.8.4 e `freedesktop-icons` 0.4 (sem release desde abr. 2025) [I3] | Igual | Igual | Igual |
| **Jump List** (menu do programa) | Ações do `.desktop` (`Actions=`) [I1]; recentes por programa não têm equivalente | Igual | Igual | Igual |
| **Painéis do sistema** (Win+A/Win+N), bateria/rede/volume | UPower, NetworkManager, PipeWire/PulseAudio por D-Bus; abrir o painel nativo varia por ambiente | Igual | Igual | Igual |
| **Avisos** (toasts) | `org.freedesktop.Notifications` (o Tauri tem plugin) | Igual | Igual | Igual |
| **Arrastar e soltar** no dock, abrir com | XDND / `wl_data_device`: vem do toolkit de janela escolhido | Igual | Igual | — |
| **WebView** (painéis, Configurações) | WebKitGTK 4.1 no Tauri [V1]; pacotes `libwebkit2gtk-4.1` e `libayatana-appindicator3` | Igual; problemas de GPU conhecidos, sobretudo NVIDIA ("Error 71", janela branca), contornados por variáveis `WEBKIT_DISABLE_DMABUF_RENDERER` etc. [V2] | Igual | Igual |

O que não tem equivalente nenhum: a **barra de tarefas a esconder e devolver** (ADR 0009) — no Linux o dock não substitui uma barra do sistema; o usuário desliga o painel do ambiente — os **perfis do Chrome** lidos pela janela, e a recuperação do processo do WebView2 e a meta de memória (são do WebView2).

### 2.3 Tauri, WebKitGTK e empacotamento

- O Tauri 2 no Linux usa WebKitGTK 4.1, que é GTK3 [V1]. O consumo de memória do WebKitGTK não foi medido aqui; é de esperar que não haja o equivalente da meta de memória do WebView2 ([spec 0003](../specs/0003-fase-2-widgets-e-persistencia.md) §2.12).
- O Tauri não tem layer-shell: não achei recurso nem issue no `tao`; no `winit` o pedido está aberto [W11]. Há um plugin de terceiros (`tauri-plugin-widgets`) com uma feature de `gtk-layer-shell` [W12]. Como a janela do Tauri é GTK3, chamar o `gtk-layer-shell` nela antes de mostrá-la é viável em tese, mas o crate GTK3 está marcado sem manutenção [W13] e a biblioteca C está "em modo de manutenção" [W14]. Não testado.
- O `tauri-bundler` gera `.deb`, `.rpm` e AppImage; Snap e AUR por fora [P1]. Flatpak é manual, com um manifesto que desempacota o `.deb` sobre o runtime `org.gnome.Platform` [P2]. Um dock em Flatpak sofre: a sandbox atrapalha D-Bus de sessão (bandeja, MPRIS), ler `.desktop` dos outros apps e os protocolos privilegiados; o pacote natural seria `.deb`/`.rpm`/AppImage.

### 2.4 Quem já fez

- **Cairo-Dock 3.6** (out. 2025): ganhou Wayland via layer-shell, testado em Wayfire, KWin, Labwc, COSMIC, Sway e Hyprland; não funciona no GNOME e ainda sem atalhos globais [D2]. É o retrato mais fiel do caminho (a).
- **nwg-dock** (Go, GTK3, gtk-layer-shell, só Sway) e **nwg-dock-hyprland** (lista de janelas pelo IPC do Hyprland) [D3][D4]: um dock por compositor.
- **Plank**: o original parou; o fork Plank Reloaded é ativo e só X11 [D5].
- **Latte Dock** (KDE): sem mantenedor desde 2022, quebrado no Plasma 6 [D6].
- **COSMIC**: dock é o `cosmic-panel` + `cosmic-app-list`, peças do próprio ambiente [D7].
- **Dash to Dock** / **Dash to Panel** / **Zorin Taskbar**: extensões do GNOME Shell [D1][Z5].

## 3. Caminhos possíveis

As estimativas são em semanas de trabalho de uma pessoa no ritmo das fases A–F, contando teste manual num segundo computador (ou máquina virtual) com cada ambiente. São ordem de grandeza, não compromisso.

### 3.0 Passo comum a todos: separar o Windows atrás de traits

Antes de qualquer Linux: tirar do domínio as suposições do Windows (identidade do programa como tipo próprio em vez de AUMID; atalho como tecla lógica em vez de VK; caminho de programa como "lançável" em vez de `.exe`/`.lnk`) e criar as "ports" que a arquitetura já previa (`WindowManager`, `MediaSessionProvider`, `SecretStore`, `Http`, `TrayHost`, `AppCatalog`, `Autostart`). Mais um job de CI em `ubuntu-latest` que só compila (`cargo check`) com os stubs.

- **Esforço:** 3–4 semanas.
- **Risco:** baixo, mas mexe em código que funciona; precisa dos testes de hoje como rede.
- **Vale sem Linux?** Em parte: os testes do domínio ficam mais limpos; o resto é custo só para o Linux.

### (a) DockMax-Linux para KDE Plasma e wlroots, com layer-shell

O dock vira uma janela layer-shell desenhada em Rust puro (`smithay-client-toolkit` + `tiny-skia` + `cosmic-text`), no lugar do `native_dock` Direct2D, com a mesma ideia da [spec 0011](../specs/0011-arquitetura-hibrida.md): o dock sem navegador, a WebView só para painéis e Configurações. Janelas pelo `wlr-foreign-toplevel` (wlroots) e pelo `plasma-window-management` (KDE, pedindo a interface no `.desktop`). Bandeja SNI, MPRIS, Secret Service, portal de atalhos, `.desktop` e ícones do tema, `reqwest`+`rustls`.

- **Esforço:** 12–16 semanas depois do passo comum. O grosso: refazer o desenho do dock, do lançador, dos menus e da agenda (~14 mil linhas hoje em Direct2D), e dois back-ends de janelas.
- **Fica de fora:** GNOME (logo Zorin e Ubuntu padrão); prévias ao vivo no KDE e no Hyprland/niri (sem protocolo aberto: ícone no lugar da miniatura); espaços de trabalho (cada compositor tem o seu); Jump List com recentes; atalho global no Sway (portal wlr não implementa); os painéis do Windows.
- **Riscos:** o `plasma-window-management` é privilegiado e pode mudar a regra de acesso; dois back-ends de janelas mais o X11 por XWayland para testar; os painéis WebView precisam se posicionar sobre o dock, e uma janela comum do Tauri **não escolhe a própria posição no Wayland** — os painéis teriam que ser layer-shell também (o hack GTK3 do §2.3) ou popups `xdg_popup` do dock, o que a WebView do Tauri não faz.

### (b) Só X11

Uma janela X11 de tipo dock com strut (EWMH), desenhada como em (a), janelas por EWMH, prévias por XComposite.

- **Esforço:** 10–12 semanas depois do passo comum. O X11 é mais parecido com o Win32 (janelas com posição, tudo visível a todos), então `openwin`/`open_apps`/`window_spaces` se traduzem quase um para um, e os painéis Tauri se posicionam como no Windows.
- **Fica de fora:** todo Wayland. Funcionaria no Zorin 18 só se o usuário escolher a sessão Xorg, e o GNOME 50 já não tem essa sessão [Z7].
- **Riscos:** construir sobre o que está saindo; tela com escala fracionária e vários monitores no X11 são fracos.

### (c) O dock desenhado pela WebView (Tauri/WebKitGTK) em vez de Direct2D

Voltar ao dock em React (como antes da spec 0011) e usar o mesmo código de desenho nos dois sistemas; no Linux, a janela do Tauri ganharia layer-shell pelo hack GTK3.

- **Esforço:** 8–10 semanas depois do passo comum para o desenho (o front já desenha boa parte em React), mais os back-ends de janelas de (a) ou (b), que não mudam.
- **Fica de fora:** o mesmo de (a) ou (b), conforme o ambiente.
- **Riscos:** é o que a [spec 0011](../specs/0011-arquitetura-hibrida.md) e o [ADR 0011](../adr/0011-spike-dock-nativo.md) deixaram para trás por memória e por GPU; o WebKitGTK tem os problemas de GPU do §2.3 e transparência que falha em algumas placas [V3]; depende de `gtk-layer-shell` GTK3 sem manutenção. Também obriga o Windows a voltar ao dock WebView, ou a manter dois desenhos de novo.

### (d) GNOME/Zorin: uma extensão do GNOME Shell

O único jeito de ter um dock de verdade no Zorin em Wayland: uma extensão em JavaScript (GJS) dentro do Shell, como o Dash to Dock. O DockMax-Linux viraria um serviço (D-Bus) com os dados e as Configurações, e a extensão desenharia.

- **Esforço:** 8–12 semanas, num código à parte, em outra linguagem.
- **Fica de fora:** quase todo o reaproveitamento do Rust de desenho; cada versão do GNOME pode quebrar a extensão.
- **Riscos:** mantém-se uma segunda base de código para sempre; o próprio Zorin já tem o Zorin Taskbar ali.

## 4. Comparação

| | Passo comum | (a) KDE + wlroots | (b) X11 | (c) WebView | (d) Extensão GNOME |
|---|---|---|---|---|---|
| Esforço (semanas) | 3–4 | +12–16 | +10–12 | +8–10 e back-ends | 8–12, à parte |
| Zorin / Ubuntu padrão | — | não | só na sessão Xorg | não (como a ou b) | sim |
| KDE Plasma | — | sim | só Xorg | sim (hack GTK3) | não |
| Sway / Hyprland | — | sim | não | sim (hack GTK3) | não |
| Prévias ao vivo | — | Sway/Labwc/COSMIC | sim | como a ou b | sim |
| Futuro | — | o caminho que a comunidade seguiu (Cairo-Dock 3.6) | encolhendo | contra a decisão da spec 0011 | atrelado a cada GNOME |

## 5. Recomendação

1. **Não portar agora.** O Linux custa de 4 a 5 meses de trabalho e não alcança o Zorin, que é o Linux que o autor usa como referência.
2. **Se o Linux virar meta**, fazer o **passo comum** e depois o **caminho (a)**: KDE Plasma + wlroots com layer-shell, dock em Rust puro (tiny-skia), sem prévias onde não há protocolo. É o que o Cairo-Dock fez em 2025 e mantém a decisão da spec 0011 (dock sem navegador). Painéis WebView sobre o dock são o maior risco técnico e pedem um spike de 1 semana antes de começar (Tauri + `gtk-layer-shell` numa VM com KDE e com Sway).
3. **GNOME/Zorin** só com a extensão (d), como projeto separado, se fizer sentido depois de (a).
4. Mesmo sem Linux, uma parte do passo comum vale a pena: tirar AUMID, VK e `.exe`/`.lnk` do domínio quando alguma fase mexer nesses arquivos. O job de CI em Linux e as traits só se pagam se o porte vier.

## 6. O que precisa de decisão do autor

- **Vale o custo?** O Linux é para uso próprio (então qual distribuição/ambiente, de fato?) ou para divulgar o DockMax (então KDE/wlroots têm o público certo)?
- **Zorin/GNOME é obrigatório?** Se for, só (d) ou (b) chegam lá, e (b) morre com o fim do X11 no GNOME.
- **Sem prévias e sem espaços de trabalho** no KDE/Hyprland é aceitável numa primeira versão?
- **Rede com `reqwest`+`rustls`** no binário do Linux, contra a regra de "nenhuma biblioteca de rede" do ADR 0012 (no Windows continua WinHTTP)?
- **Empacotamento:** `.deb` + AppImage (sem Flatpak, que atrapalha um dock) está bom?
- **Quando:** depois da fase G inteira, ou como fase própria com spec?

## Fontes

Consultadas em 2026-10-29. "Não confirmado" no texto marca o que nenhuma fonte acima sustentou.

- [Z1] Zorin OS 18 lançado, base Ubuntu 24.04 — https://ubuntuhandbook.org/index.php/2025/10/zorin-os-18-officially-released-based-on-ubuntu-24-04-lts/
- [Z2] Zorin OS 18 — https://www.omgubuntu.co.uk/2025/10/zorin-os-18-released-with-new-look-new-apps-more
- [Z3] Zorin OS 18.1 — https://www.theregister.com/2026/04/21/zorin_os_181_released/
- [Z4] Sessões do Zorin 18 (fórum) — https://forum.zorin.com/t/does-zorin-18-come-with-wayland-and-x11-like-17/53763
- [Z5] Zorin e Dash to Panel — https://www.theregister.com/2025/03/30/zorin_and_dash_to_panel/
- [Z6] Ubuntu 25.10 sem sessão Xorg — https://www.omgubuntu.co.uk/2025/06/ubuntu-25-10-dropping-xorg-support
- [Z7] GNOME 50 só Wayland — https://www.theregister.com/2026/03/19/gnome_50/
- [W1] wlr-layer-shell e compositores — https://wayland.app/protocols/wlr-layer-shell-unstable-v1
- [W2] gtk4-layer-shell — https://crates.io/crates/gtk4-layer-shell
- [W3] smithay-client-toolkit, `shell::wlr_layer` — https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/shell/wlr_layer/index.html
- [W4] wlr-foreign-toplevel-management — https://wayland.app/protocols/wlr-foreign-toplevel-management-unstable-v1
- [W5] ext-foreign-toplevel-list-v1 — https://wayland.app/protocols/ext-foreign-toplevel-list-v1
- [W6] kde-plasma-window-management — https://wayland.app/protocols/kde-plasma-window-management
- [W7] KWin, `X-KDE-Wayland-Interfaces` — https://mail.kde.org/pipermail/kwin/2019-July/000625.html
- [W8] KWin, dock de terceiros (2024) — https://mail.kde.org/pipermail/kwin/2024-January/005305.html
- [W9] ext-image-copy-capture-v1 — https://wayland.app/protocols/ext-image-copy-capture-v1
- [W10] ext-image-capture-source-v1 — https://wayland.app/protocols/ext-image-capture-source-v1
- [W11] winit, pedido de layer-shell — https://github.com/rust-windowing/winit/issues/2582
- [W12] tauri-plugin-widgets (gtk-layer-shell) — https://docs.rs/crate/tauri-plugin-widgets/0.4.3/source/docs/contributing/linux-harness.md
- [W13] gtk-layer-shell (crate GTK3, sem manutenção) — https://crates.io/crates/gtk-layer-shell
- [W14] gtk-layer-shell (biblioteca C) — https://github.com/wmww/gtk-layer-shell
- [X1] EWMH — https://specifications.freedesktop.org/wm-spec/latest/
- [X2] x11rb — https://crates.io/crates/x11rb
- [R1] tiny-skia — https://crates.io/crates/tiny-skia
- [R2] vello — https://crates.io/crates/vello ; wgpu — https://crates.io/crates/wgpu
- [R3] skia-safe — https://crates.io/crates/skia-safe
- [R4] femtovg — https://crates.io/crates/femtovg
- [R5] cosmic-text — https://crates.io/crates/cosmic-text
- [T1] StatusNotifierWatcher / Host — https://specifications.freedesktop.org/status-notifier-item/0.1/status-notifier-watcher.html
- [T2] system-tray (host SNI) — https://crates.io/crates/system-tray
- [M1] MPRIS2 — https://specifications.freedesktop.org/mpris/latest/
- [M2] mpris — https://crates.io/crates/mpris
- [M3] zbus — https://crates.io/crates/zbus
- [S1] Secret Service API — https://specifications.freedesktop.org/secret-service/latest/
- [S2] keyring — https://docs.rs/keyring/latest/keyring/
- [S3] oo7 — https://crates.io/crates/oo7
- [S4] secret-service — https://crates.io/crates/secret-service
- [K1] global-hotkey ("Linux (X11 Only)") — https://docs.rs/global-hotkey/latest/global_hotkey/
- [K2] Portal GlobalShortcuts — https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html
- [K3] ashpd — https://crates.io/crates/ashpd
- [K4] Quem implementa o portal (issue do xfwl4, fonte secundária) — https://gitlab.xfce.org/xfce/xfwl4/-/issues/93
- [K5] GNOME 48 para desenvolvedores — https://release.gnome.org/48/developers/
- [A1] XDG Autostart — http://specifications.freedesktop.org/autostart/latest/
- [A2] tauri-plugin-autostart — https://crates.io/crates/tauri-plugin-autostart
- [A3] Portal Background — https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Background.html
- [I1] Desktop Entry spec — https://specifications.freedesktop.org/desktop-entry-spec/latest/
- [I2] Icon Theme spec — https://specifications.freedesktop.org/icon-theme-spec/latest/
- [I3] freedesktop-desktop-entry — https://crates.io/crates/freedesktop-desktop-entry ; freedesktop-icons — https://crates.io/crates/freedesktop-icons
- [V1] Pré-requisitos do Tauri 2 no Linux — https://v2.tauri.app/start/prerequisites/
- [V2] Tauri, gráficos no Linux — https://v2.tauri.app/develop/debug/linux-graphics/
- [V3] Janela transparente vazia no Wayland/AMD — https://github.com/block/buzz/issues/2643
- [P1] Distribuição do Tauri — https://v2.tauri.app/distribute/
- [P2] Tauri e Flatpak — https://v2.tauri.app/distribute/flatpak/
- [D1] Dash to Dock — https://github.com/micheleg/dash-to-dock
- [D2] Cairo-Dock 3.6 — https://www.phoronix.com/news/Cairo-Dock-3.6-Released
- [D3] nwg-dock — https://github.com/nwg-piotr/nwg-dock
- [D4] nwg-dock-hyprland — https://github.com/nwg-piotr/nwg-dock-hyprland
- [D5] Plank Reloaded — https://www.omgubuntu.co.uk/2025/02/plank-reloaded-linux-dock-cinnamon/amp
- [D6] Latte Dock sem mantenedor — https://mail.kde.org/pipermail/plasma-devel/2023-October/123173.html
- [D7] cosmic-panel — https://github.com/pop-os/cosmic-panel ; cosmic-applets — https://github.com/pop-os/cosmic-applets
