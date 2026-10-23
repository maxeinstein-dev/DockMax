# ADR 0009 — O dock substitui a barra de tarefas (e passa a ter licença de uso pessoal)

- **Status:** Aceita em 2026-10-23, depois do spike da barra de aplicativos (resultados no fim)
- **Data:** 2026-10-22
- **Substitui:** [ADR 0004](0004-escopo-windows.md) na parte "conviverá com a barra de tarefas nativa"

## Contexto

O autor quer que o DockMax tenha a mesma proposta de produto do PrimoDock: um Dock por workspace, apps abertos, prévias de janela, bandeja hospedada e controle da barra de tarefas do Windows (deixar visível, ocultar automaticamente ou ocultar por completo). O DockMax ainda está em MVP, então a arquitetura pode mudar. O ADR 0004 previa conviver com a barra nativa e evoluir devagar.

## Decisão

1. O dock é, em última instância, o shell de janelas do usuário. Ele registra uma **barra de aplicativos** (app bar) para reservar a área de trabalho e oferece três modos para a barra nativa: **visível** (o dock fica acima dela), **ocultar automaticamente** e **ocultar por completo**. O **padrão é ocultar por completo**, como o PrimoDock: o dock não roda junto com a barra do Windows, a menos que o usuário escolha "Visível".
2. A barra nativa **sempre volta**: qualquer saída do dock (normal, fim da sessão, panic, processo morto, queda de energia) devolve o Windows ao estado original, e a opção "Visível" é o fallback.
3. O escopo e a ordem de entrega estão na [spec 0004](../specs/0004-paridade-primodock.md). O **spike da barra de aplicativos** foi a primeira entrega de risco; seus resultados estão no fim e levaram o ADR a Aceita.
4. A **implementação é do zero**: nenhum código, executável, ícone, sprite ou catálogo do PrimoDock entra no repositório. Ícones, pacotes e pet são desenhados para o DockMax.
5. A **licença passa a ser de uso pessoal** (todos os direitos reservados), a partir da 0.3.0. O autor é o único titular do código (169 commits, um autor), o que permite a mudança. As versões até a 0.2.0 seguem MIT para quem as obteve.

## Alternativas consideradas

- Manter só a convivência com a barra (ADR 0004): não entrega a paridade pedida (workspaces com janelas próprias, bandeja no dock).
- Reescrever como aplicativo .NET/WPF: reaproveitaria bibliotecas de shell, mas descarta Tauri, React, o SQLite e os testes da Fase 2.
- Manter a licença MIT: descartada pelo autor; o projeto é de uso pessoal.

## Consequências

- Mexer na barra de tarefas e na área de trabalho exige **restauração garantida** do estado do Windows, com testes de falha (encerramento forçado, perda de sessão).
- A bandeja não tem equivalente pronto em Rust: o spike decide entre implementar o host, espelhar os ícones ou adiar.
- Prévias de janela pedem uma janela nativa auxiliar sobre o WebView2.
- Vários monitores multiplicam renderizadores: o orçamento de memória do roadmap (~220 MB) precisa ser revisto com a medição.
- `PRODUCT.md`, o roadmap e o README refletem a nova direção.

## Resultado do spike (2026-10-23)

Medido no Windows 11 real, monitor principal 2560 × 1080 a 100 % e um segundo monitor, com o PrimoDock fechado.

| Critério | Resultado |
|---|---|
| Reserva da borda | A área de trabalho passa de 1032 para 1016 px (faixa de 64 px: barra de 62 px mais 2 px até a borda). O dock fica a 2 px da borda, dentro da faixa. |
| Barra do Windows | Em "Automática", só o flag de ocultar. Em "Oculta", as barras dos **dois monitores** somem e continuam ocultas 8 s depois (o Explorer as mostra de novo se ninguém reforçar: ver abaixo). |
| Foco | O dock nunca tomou o primeiro plano, em nenhum modo, amostrado a cada 200 ms por 8 s após abrir. |
| Memória em repouso | 184,8 a 187,1 MB privados (dock, WebView2 e o vigia de 1,3 MB), contra a linha de base de 182 MB e o orçamento de 220 MB. |
| Processo morto à força | O vigia devolve a barra a um estado idêntico ao inicial (flag, janelas e área de trabalho), nos modos Automática, Oculta e padrão. |
| Fim da sessão (`WM_ENDSESSION`) | Restaurado ao estado inicial. |
| Dock e vigia mortos juntos | A barra fica alterada até a próxima abertura, que a restaura antes de qualquer coisa. |
| Padrão | Um `settings.json` sem o campo `taskbar` (o do autor) abre com a barra oculta. |

O que o spike descobriu:
- O Explorer **mostra a barra de novo** depois de mudar o flag e quando os monitores mudam. Uma única passada de ocultar deixou a barra do segundo monitor visível. A solução: algumas tentativas nos primeiros segundos e uma nova ao receber `ABN_POSCHANGED`, `WM_DISPLAYCHANGE`, `WM_SETTINGCHANGE` ou `TaskbarCreated`. Não há laço permanente.
- A janela do app bar vive numa thread própria e nunca chama o Tauri: a thread principal espera por ela, então o contrário seria um impasse. Ela só pede, com `run_on_main_thread`, que o dock se reposicione.
- O vigia é este mesmo executável com `--taskbar-watchdog <pid> <arquivo>`; `--restore-taskbar <arquivo>` restaura à mão.
- As builds de teste end-to-end nunca mexem na barra.

Consequências que continuam abertas:
- **Sem a barra, o Windows perde o relógio, a bandeja e as configurações rápidas.** O dock ainda não os substitui: o widget Relógio existe, mas a bandeja, a bateria, a rede e o volume são da Fase E.
- Só o monitor principal recebe dock; a barra do outro monitor é ocultada, mas ele fica sem dock até o suporte a vários monitores.
- Aplicativos em tela cheia (`ABN_FULLSCREENAPP`), a saída pelo menu "Sair" da bandeja e o comportamento com o menu Iniciar não foram exercitados.
- A borda em cima usa a mesma lógica e está coberta só por testes de unidade.

## Fase E: relógio, bandeja e a garantia de devolver a barra (2026-10-23)

- **Relógio e data** no fim do dock, enquanto a barra do Windows está oculta (por completo ou automática). Um clique abre o calendário do Windows (Win+N, enviado como teclas, que é tudo o que o Windows oferece).
- **Bandeja hospedada** (`windows/tray_host.rs`), só no modo "Oculta": o dock cria uma janela `Shell_TrayWnd` própria, acima da do Explorer na ordem de z, que **encaminha tudo** à janela real e apenas observa os ícones que passam. O Explorer continua funcionando, oculto e com os mesmos ícones, então nada se perde se o dock cair, e as barras de aplicativos (`SHAppBarMessage`) continuam valendo. Os programas já abertos reenviam os ícones com o broadcast `TaskbarCreated`.
- **Painel da bandeja:** item do dock que abre um painel com os ícones (imagem em PNG, dica); clique esquerdo e direito vão ao programa, como a barra mandaria, e o menu que aparece é o do próprio programa.
- **Validado ao vivo:** a sonda (`cargo run --example tray_probe`) recebeu os ícones reais (Segurança do Windows, volume, Bluetooth, WhatsApp, FortiClient e outros) com dica e imagem. Um programa de teste que se comporta como os de bandeja recebeu as mensagens exatamente como o Windows documenta, na versão antiga (`wParam` é o id) e na 4 (posição em `wParam`, evento e id em `lParam`, mais `WM_CONTEXTMENU`).
- **Menu do FortiClient (as VPNs salvas):** investigado até a causa. Quatro coisas faziam o menu não abrir ou abrir fora do lugar, todas corrigidas:
  1. **Versão do protocolo.** O FortiClient escreve "4" na estrutura de todas as mensagens, mas nunca envia `NIM_SETVERSION`; pela regra do Windows ele é um ícone da versão antiga, e eu lhe mandava o clique no formato novo, que ele ignora. A versão só vale quando vem da mensagem `NIM_SETVERSION` (verificado com um programa de teste que a envia).
  2. **Resposta de falha.** Com a barra oculta o Explorer responde "não" às mensagens de ícone que o dock lhe repassa, e o programa entendia que o ícone não foi adicionado. O dock, que é quem hospeda a bandeja, responde "sim" ao que aceitou.
  3. **Ordem das janelas.** A janela `Shell_TrayWnd` do Explorer voltava a ficar acima da do dock, e os programas que adicionavam depois mandavam o ícone para ela. O dock mantém a sua acima (`raise`) e rebaixa a do Explorer enquanto estiver oculta; a restauração a devolve ao topo.
  4. **Posição do menu.** Os programas perguntam ao Windows onde o ícone está (`Shell_NotifyIconGetRect`, duas mensagens `WM_COPYDATA` tipo 3, com a resposta em `x | y << 16` e `largura | altura << 16`). Com a barra oculta o Explorer respondia (2314, 0), o canto superior direito, e o menu abria ali. O dock responde com um quadrado de 32 px em volta do último clique, por 5 s.
- **O painel fica aberto no clique direito** (`windows/popup_menu.rs`): o menu do programa toma o foco e o painel se escondia ao perdê-lo. Agora o clique direito segura o painel até o menu acabar. O Windows avisa quando um menu começa e termina (`EVENT_SYSTEM_MENUPOPUPSTART` e `END`, um par por submenu); se nenhum menu começa em 4 s, a retenção termina sozinha. Quando o menu acaba, o painel recebe o foco de volta, para o Esc e o clique fora fecharem como sempre; se o usuário clicou em outra janela, o painel fecha. O clique esquerdo continua fechando o painel, porque abre uma janela do programa.
- **Verificado ao vivo** (mouse real, captura da tela inteira, dois monitores): o clique direito no ícone do FortiClient abre o menu dele ("Open FortiClient Console", "About FortiClient", "Connect to ..." para cada VPN salva, "Shutdown FortiClient") acima do ícone clicado, com o painel da bandeja aberto; depois de fechar o menu com Esc, o painel continua aberto.
- Os ícones chegam em tempos diferentes depois do `TaskbarCreated` (o FortiClient apaga e recria o dele): o aviso é repetido 4 vezes nos primeiros 16 s.
- Rastreio para investigar: com `DOCKMAX_TRAY_TRACE=1` o dock escreve no log cada mensagem de ícone que passa e cada pergunta de posição; a sonda (`cargo run --example tray_probe -- [segundos] [parte da dica] [left|right]`) roda o hospedeiro sozinho.
- **Menus dos aplicativos do dock** (Jump Lists) ficam para a Fase B; o requisito está na spec 0004.

### A barra volta como estava, em todos os casos

O autor viu a barra ficar em "ocultar automaticamente" depois de fechar o DockMax. A causa: o estado ficava gravado com um processo que morreu, porque o vigia era uma cópia do próprio `dockmax.exe`, filha do app, e **morria junto** com a árvore de processos (o "Finalizar tarefa" do Gerenciador de Tarefas, `taskkill /T`) ou com um `taskkill /IM dockmax.exe`. A correção:

- o vigia é uma **cópia com outro nome** (`dockmax-guard.exe`, na pasta de dados), então encerrar `dockmax.exe` pelo nome não o alcança;
- ele é iniciado por um `cmd` que termina logo, então o vigia fica **fora da árvore** do app;
- o fim de sessão do Windows também desliga a bandeja hospedada.

Verificado: a barra volta a um estado idêntico ao inicial (flag, janelas e área de trabalho) ao matar só o processo, ao matar a árvore inteira (`taskkill /F /T`) e ao matar por nome (`taskkill /F /IM dockmax.exe`). Continua valendo que, se o vigia e o app forem mortos juntos de propósito (por exemplo, o vigia primeiro), a barra só volta na próxima abertura do DockMax ou com `dockmax-guard.exe --restore-taskbar <arquivo de estado>`.
