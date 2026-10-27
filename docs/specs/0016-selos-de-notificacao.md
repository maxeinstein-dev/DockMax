# Spec 0016 — Selos de notificação

Fora das fases da [spec 0004](0004-paridade-primodock.md): pedido do autor depois do spike de 2026-10-27. É a bolinha com o número de não lidas que o celular põe no ícone do app (WhatsApp, Telegram, Outlook, Teams), desenhada nos ícones do dock.

## 1. De onde vem o número

O Windows não tem uma API pública que dê o selo de outro programa: o app o põe no seu botão da barra de tarefas (`ITaskbarList3::SetOverlayIcon`, ou a badge de um app da Loja), e só a barra o sabe. O DockMax o **lê da barra pela UI Automation**, como um leitor de tela faria.

O que o spike mostrou (2026-10-27, PowerShell e UI Automation) e o que este recorte confirmou:

- Na `Shell_TrayWnd` do Explorer, cada programa é um botão de classe `Taskbar.TaskListButtonAutomationPeer`. O `Name` é o nome do programa e o que a barra diz dele (`"WhatsApp - 1 janela em execução fixado"`).
- **O `AutomationId` do botão é `"Appid: <AppID>"`** (`Appid: 5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App`, `Appid: Chrome`, `Appid: Chrome.UserData.Profile1`). É por ele que o botão se liga ao app do dock, e não pelo nome.
- O número é um texto sob o botão (um `TextBlock`, com `Name` = `"1"`).
- A barra **escondida com `SW_HIDE`** (como o DockMax fazia, ADR 0009) não mostra nenhum botão na UI Automation. A barra **visível e transparente** mostra os botões e o número.
- A barra secundária (`Shell_SecondaryTrayWnd`) mostra os mesmos botões, com o mesmo `AutomationId`; basta ler a do monitor principal.

## 2. A barra fica transparente, não escondida

Com "Barra do Windows: oculta" (o padrão), o DockMax deixa de esconder a barra com `SW_HIDE` e passa a:

- pôr `WS_EX_LAYERED` com alfa 0 (invisível) e `WS_EX_TRANSPARENT` (os cliques a atravessam) em cada janela da barra, nos dois monitores;
- manter a ocultação automática da barra ligada, como já fazia: ela não reserva espaço e só "sobe" quando o ponteiro encosta na borda, invisível e sem pegar o clique;
- baixá-la para baixo das janelas sempre no topo, como já fazia, para que o `Shell_TrayWnd` que os programas acham primeiro seja o proxy da bandeja do DockMax (`tray_host.rs`).

A devolução segue o ADR 0009, por todos os caminhos (fechar, queda, vigia `dockmax-guard.exe`, `--restore-taskbar`, recuperação na partida seguinte):

- **Antes de mexer**, o `taskbar-state.json` ganha, para cada janela, o **estilo estendido original** e, se ela já era em camadas, os atributos de camada (cor-chave, alfa, flags). Uma janela só entra no arquivo uma vez: o que ela é depois pode ser obra do próprio DockMax.
- Ao devolver: o estilo original volta, os atributos de camada voltam se havia, e a janela é repintada; ela volta a ficar sempre no topo; a ocultação automática volta a ser a do usuário.
- **Arquivos de versões anteriores** (com `hidden`, sem `faded`) ainda são lidos e devolvidos: as janelas escondidas são mostradas de novo.
- O vigia de sempre (`keep`) refaz a transparência quando o Explorer a desfaz (mudança de tema, de monitor, reinício do Explorer).

Os modos "Ocultar automaticamente" e "Visível" não mudam: neles a barra já está visível e os selos são lidos do mesmo jeito.

## 3. Leitura

- **Uma thread própria** (`windows/badges.rs`), em MTA, com o `IUIAutomation`. Ela acha a `Shell_TrayWnd` do Explorer (nunca o proxy do DockMax, que tem a mesma classe: a busca descarta as janelas do próprio processo) e lê, numa chamada só ao Explorer (`FindAllBuildCache`, com um cache de `Name` e `AutomationId` da subárvore de cada botão), os botões e os textos sob eles.
- **Sem polling caro**: a thread se inscreve nos eventos de mudança de estrutura e de `Name` da subárvore da barra; uma rajada de eventos vira uma leitura, 250 ms depois do último (no máximo 1,5 s esperando). Uma **batida lenta, a cada 10 s**, lê de novo caso um evento não venha e acha uma barra que o Explorer recriou.
- O dock só é avisado quando o que foi lido mudou, e redesenha só se os seus selos mudaram.

### 3.1 O que é um selo

Os textos sob o botão, o primeiro que for um selo:

- um número (`"3"`, ou `"99+"` se a barra abreviar) é uma **contagem**; zero não é selo;
- uma marca curta que não é palavra (`"!"`, `"•"`, até dois caracteres sem letra nem dígito) é um **selo sem número**;
- qualquer outra coisa (o título de uma janela, quando a barra mostra rótulos) não é selo.

### 3.2 A que item do dock o selo pertence

O mesmo critério de `domain/open_apps.rs`:

- **Programa fixado:** o botão cujo AppID é um dos AppIDs do programa (o id da pasta Apps e o `System.AppUserModel.ID`), ou cujo AppID foi feito do arquivo do programa (`{GUID da pasta conhecida}\pasta\programa.exe`, a pasta resolvida) e aponta para o mesmo arquivo.
- **App aberto não fixado:** o botão cujo AppID (ou arquivo) é aquilo pelo qual as janelas dele são agrupadas. Os perfis do Chrome têm AppIDs próprios na barra, como no dock.
- **Botão sem AppID** (não visto no spike; só por garantia): o nome do botão é o nome de exibição do app, sozinho ou seguido de `" - "`.

### 3.3 Espaços de trabalho

Cada dock mostra os itens do espaço do seu monitor (spec 0013), e o selo vai nesses itens: um programa fixado no espaço mostrado, ou um app aberto com janela nesse espaço (`open_apps::snapshot_for`). Um app que não está no espaço mostrado não aparece, nem o seu selo. **Proposta, valendo até o autor decidir.**

## 4. Desenho

- Uma pílula vermelha (a cor `danger` da paleta) no canto superior direito do ícone, passando 3 px dele, com o número em branco e um anel da cor da barra em volta, como no celular. A altura é 38 % do ícone, entre 14 e 22 px; o número alarga a pílula para a esquerda.
- **"99+"** acima de 99. Um selo sem número é um ponto menor, sem texto.
- Nos dois monitores, acompanhando o ícone quando ele cresce sob o ponteiro, é arrastado ou anima ao abrir. Não aparece no reflexo.
- A geometria é pura, em `domain/dock_layout.rs` (`badge_area`).

## 5. Configurações

Uma chave só, **"Mostrar selos de notificação"**, ligada por padrão, junto de "Mostrar apps abertos" (o visual de Configurações fica para o rebranding). Desligada, o dock para de desenhar os selos; a thread de leitura só começa quando a chave está ligada. **Proposta, valendo até o autor decidir.**

## 6. Teste ao vivo (2026-10-27): bloqueio, decisão do autor

**A barra transparente só tem botões na UI Automation enquanto não está recolhida.** Medido sem o DockMax, na barra principal:

| Estado da barra                           | Botões na UIA |
| ----------------------------------------- | ------------- |
| normal                                    | 6             |
| transparente, sem ocultação automática    | 6             |
| transparente, ocultação automática recolhida | **0**      |

Recolhida, a barra do Windows 11 desmonta o seu conteúdo XAML: sobram só `TrayNotifyWnd`, `ReBarWindow32` e `MSTaskSwWClass`, sem nenhum botão. E é a ocultação automática que faz a barra não reservar espaço. Do jeito que está, **no modo "oculta" o DockMax não acha selo nenhum**. Os selos funcionam no modo "Visível". `WS_EX_TRANSPARENT` não tem nada a ver: tirá-lo não traz os botões de volta.

Caminhos para o autor escolher:

1. **Barra transparente sem ocultação automática:** os selos funcionam, mas a barra reserva os 48 px dela, e o dock fica acima deles ou passa a usar essa faixa.
2. **Voltar a esconder com `SW_HIDE`** (o código anterior a esta spec) e ter selos só no modo "Visível".
3. **Investigar** um jeito invisível de "subir" a barra recolhida de tempos em tempos para ler. Não sei se existe sem roubar o foco.

**Proposta, valendo até o autor decidir:** o PR fica aberto, sem merge, até a escolha. A troca para transparente só vale a pena com o caminho 1 ou 3.

Também visto no teste:

- **Devolução:** pelo fim de sessão (`WM_ENDSESSION`) a barra volta exatamente como era nos dois monitores (estilo, camadas, visibilidade, ocultação automática, topo). Pelo vigia, depois de o DockMax ser morto à força, o estilo, as camadas e a ocultação automática voltam, mas **a barra principal não volta a ficar sempre no topo**: o Windows ignora o `SetWindowPos(HWND_TOPMOST)` vindo de um processo sem direito de primeiro plano, inclusive o vigia. Isso já valia para o código anterior, que também baixava a barra; ela só voltou ao topo quando um DockMax vivo fechou depois. Fica como pendência à parte.
- **A barra secundária não é sempre no topo** (`WS_EX_TOOLWINDOW` sem `WS_EX_TOPMOST`): a devolução agora só põe no topo quem estava.
- **Memória** (30 s depois de abrir, os dois monitores): 85,0 MB privados com os selos contra 84,1 MB do build da main; mais 7 threads (as da UI Automation). A leitura custa ~40 ms de uma chamada ao Explorer.
- `--restore-taskbar` não foi testado ao vivo, só nos testes de unidade do arquivo de estado.

## 7. Ficou para depois / não verificado

- **Se o número muda com a barra transparente** sem nenhuma outra mudança na barra: os eventos da UI Automation chegam com a janela invisível? A batida de 10 s cobre o caso, com até 10 s de atraso. Ver o relato do teste ao vivo no PR.
- O selo na UI Automation do dock (o leitor de tela ainda não o anuncia).
- Selo nos itens da bandeja do dock.
- O formato exato dos selos sem número de cada app (ícones de status do Teams, por exemplo) não foi visto; podem vir sem texto, e aí não aparecem.
