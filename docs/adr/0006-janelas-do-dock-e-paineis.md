# ADR 0006 — Janelas do dock e painéis

- **Status:** Aceita
- **Data:** 2026-10-14

## Contexto

O dock precisa flutuar acima da barra de tarefas, sem atrapalhar os apps de trás, abrir painéis ancorados aos itens e, se possível, ampliar ícones ao passar o mouse e ter aparência de vidro. A spec 0002 deixou três pontos para validação empírica no Windows real (V1 ampliação, V2 vidro, V3 Alt+Tab). Este ADR registra o desenho adotado e o que as validações mostraram.

## Decisão

### Duas janelas

- **`dock`**: sem borda, transparente, sempre no topo, sem redimensionar, fora da barra de tarefas e do Alt+Tab. Tem a barra visível (64 px lógicos) mais uma **folga transparente de 40 px** do lado oposto à borda da tela, para a ampliação. A borda de fora da barra fica a 8 px da área útil do monitor principal.
- **`panel`**: uma única janela, criada oculta e reaproveitada. Abre ancorada ao item clicado, 8 px além da barra visível, contida na área útil. Fecha com Esc, com perda de foco, com novo clique no item e com Alt+F4.
- Uma janela transparente grande e única foi descartada: no Windows, a área transparente de uma WebView bloqueia o clique para os apps de trás.

### Comportamento nativo

- **Alt+Tab (V3):** o `skipTaskbar` do Tauri só remove o botão da barra de tarefas, e o tao reescreve o estilo estendido inteiro a cada mudança. Um subclass Win32 (`windows/native.rs`) força `WS_EX_TOOLWINDOW` e remove `WS_EX_APPWINDOW` em `WM_STYLECHANGING`, nas duas janelas.
- **Reposicionamento:** o Tauri não emite evento para mudança de resolução ou de área útil, e o `ScaleFactorChanged` chega antes de o tao mover a janela. O mesmo subclass reage a `WM_DPICHANGED`, `WM_DISPLAYCHANGE` e `WM_SETTINGCHANGE(SPI_SETWORKAREA)` e reposiciona depois, via `WM_APP+1`. O cálculo de posição é uma função pura com testes para 100/125/150/175%.
- **Foco:** o `show()` do tao sempre ativa a janela, e o `set_focus` simula um Alt para roubar o primeiro plano. O dock aparece sem ativar (`focus: false` + `set_focusable(false)` só durante o `show`), e o painel só recebe foco quando o DockMax já está em primeiro plano, o que acontece no clique real no dock.
- **Bandeja:** "Mostrar/ocultar dock" e "Sair". Alt+F4 no dock oculta; só "Sair" encerra.

### Ampliação com clique atravessando a folga (V1: viável)

- A janela do dock ignora o mouse por padrão. Uma thread lê `GetCursorPos` a cada 16 ms e alimenta uma função pura com histerese: captura quando o cursor entra na barra; mantém capturado enquanto ele estiver dentro da janela; libera ao sair. `set_ignore_cursor_events` só é chamado nas transições. Com o dock oculto, a thread fica parada.
- O frontend informa o retângulo da barra pelo comando `set_dock_hit_rect`.
- Medido: CPU em repouso ~0,03–0,16% de um núcleo; `WindowFromPoint` na folga e na barra, em repouso, devolve o app de trás.
- Consequência aceita: depois de entrar pela barra, a folga inteira captura cliques até o cursor sair da janela.

### Vidro simulado (V2: desfoque real inviável)

- Os backdrops do DWM (Acrylic via `DWMSBT_TRANSIENTWINDOW`, Acrylic legado e Blur) cobrem o **retângulo inteiro** da janela e **ignoram** `SetWindowRgn`: o resultado é sempre uma caixa de cantos retos. No dock, cobriria também a folga da ampliação; com região recortada, o ícone ampliado some.
- A preferência de canto do DWM só arredonda ~8 px, o que a spec descartou.
- Com "Efeitos de transparência" desligado (caso desta máquina), economia de bateria ou janela inativa, o Acrylic vira cor sólida, e o dock nunca fica ativo.
- Adotado: **vidro simulado em CSS** nas duas janelas (camada tingida translúcida, brilho sutil, borda clara de 1 px, raio de 20 px), com a opacidade num token (`--glass-opacity`), de modo que a opção "Sólido" é a troca de um valor.

### Permissões por janela

- Os comandos do app são declarados no `AppManifest` do `tauri_build` e exigem permissão explícita. `dock` pode chamar `toggle_panel` e `set_dock_hit_rect`; `panel` só `hide_panel`. Chamadas fora disso falham com `Command … not allowed by ACL`.

### CSP

- Em release: `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:`, sem `unsafe-eval`, sem `unsafe-inline` e sem curingas. O Tauri acrescenta `script-src` com o hash do bundle.
- Em `pnpm tauri dev` no desktop **não há CSP**: a WebView carrega direto do servidor do Vite. Problemas de CSP só aparecem no build de release, que deve ser testado antes de cada PR.

## Alternativas consideradas

- **Uma janela única que cresce com o painel:** descartada pela área transparente que bloqueia cliques.
- **Desfoque real com duas camadas** (janela só de backdrop sob a janela de ícones): herdaria a caixa de cantos retos, exigiria janela sem WebView (recurso `unstable`) e acoplamento de ordem e posição. Esforço médio, risco alto, sem ganho.
- **Gancho global de mouse (`WH_MOUSE_LL`)** em vez de leitura periódica do cursor: mais invasivo e sensível a privacidade para um ganho desprezível, dado o custo medido.

## Consequências

- Comportamentos que dependem do Win32 ficam isolados em `windows/native.rs`, sob `#[cfg(windows)]`.
- O painel aberto se sobrepõe a parte da folga do dock; ícones ampliados sob o painel ficam cobertos. A ampliação também sobrepõe vizinhos (o `transform` não empurra o layout). Ambos são tratados no visual do PR 3.
- O "vidro" não desfoca o fundo; a legibilidade vem da opacidade, ajustável pelo usuário (Translúcido/Sólido).
- Testar CSP exige build de release.
