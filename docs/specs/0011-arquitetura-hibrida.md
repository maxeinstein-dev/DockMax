# Spec 0011 — Arquitetura híbrida: dock nativo, WebView2 sob demanda

Decisão do autor (2026-10-26), a partir do [ADR 0011](../adr/0011-spike-dock-nativo.md): o dock passa a ser desenhado em Rust nativo e o WebView2 só é carregado quando um painel ou as Configurações abrem.

## 1. Recortes

| Recorte | Entrega |
|---|---|
| **H1** | **App de Configurações**: uma janela própria, criada quando abre e destruída quando fecha. Corrige o layout quebrado de "Aparência do dock" no painel (o cartão de 320 px cortava as linhas) |
| **H2** | **Painéis sob demanda**: a janela do painel (widgets, menus, bandeja, Sobre) é criada ao abrir e destruída depois; com o dock nativo, nada do WebView2 fica carregado em repouso |
| **H3** | **Dock nativo** (Direct2D), em partes: itens, ampliação e clique; estilos de barra; faces dos widgets; arrastar e soltar; rótulos, indicador e acessibilidade |
| **D4b** | Um dock nativo por monitor ([spec 0010](0010-fase-d4-monitores-e-tela-cheia.md)) |

## 2. Quanto custa abrir o WebView2 sob demanda

Medido com `spikes/webview-startup` (2026-10-26): uma janela WebView2 nova carregando a página real de Configurações (o bundle do DockMax, servido como o Tauri serve), do pedido até a página carregada e pintada.

| Situação | Tempo até a página pintada |
|---|---|
| **Frio**: nenhum WebView2 do processo rodando (o processo do navegador já tinha saído), como depois do repouso no híbrido | 354–363 ms (três medições) |
| **Morno**: outro WebView2 do processo aberto | 110–140 ms (três medições) |
| Hoje: o painel já carregado, só mostrado | dezenas de ms (a página só re-renderiza) |

Os testes de ponta a ponta rodavam ao mesmo tempo, então os números podem estar um pouco altos. Uma página de verdade lê os seus dados ao abrir, o que soma alguns milissegundos.

**O que isso significa:** abrir um painel depois do repouso fica perceptível (~⅓ de segundo), mas não longo. Para o H2 ficam duas mitigações, a decidir com a medição do próprio H2:

- **Aquecer ao passar o mouse:** começar a criar a janela do painel quando o ponteiro para sobre um item que abre painel. O clique costuma vir de 200 a 400 ms depois, o que esconde quase todo o tempo frio.
- **Manter morno por alguns minutos** depois de fechar um painel, e só então deixar o WebView2 sair.

## 3. H1 em detalhe

- **A janela:** `settings` (`windows/settings_window.rs`), uma janela comum com barra de título, 880 × 640 (mínimo 720 × 480), centralizada, com os mesmos argumentos do WebView2 das outras (um teste garante). É criada **fora da thread principal**: criar uma janela WebView2 de dentro de um comando ou do menu da bandeja (que rodam na thread principal) trava o app, como o teste no app real mostrou. Abrir de novo traz a que está aberta para a frente.
- **Quem abre:** "Configurações" no menu do dock (botão direito fora de um programa) e no menu do ícone da bandeja. Os dois menus têm o mesmo conteúdo: Gerenciador de Tarefas (como no menu da barra do Windows), Configurações, **Ocultar automaticamente** (com a marca de ligado) e Sair.
- **Ocultar automaticamente** (`dock.autoHide`, desligado por padrão; também em Configurações, com os dois tempos): o dock desliza para fora da tela quando o ponteiro fica fora dele pelo **atraso para ocultar** (`autoHideDelay`, 0,7 s, até 5 s) e volta quando o ponteiro encosta na borda do monitor dele pelo **atraso para mostrar** (`autoHideReveal`, 0,3 s, até 2 s). Não desce com um painel ou um menu aberto, nem durante um arrasto. A faixa reservada é devolvida aos programas, como na barra do Windows com ocultar automaticamente. Abrir o DockMax de novo (pelo menu Iniciar) traz o dock de volta. Inspirado no dock do ZorinOS.
- **A página** (`settings.html`, `src/settings/`): barra lateral com Geral, Aparência do dock, Apps do dock, Itens do dock e Sobre, e ao lado a mesma tela que o painel tinha, sem o "Voltar". Esc fecha a janela.
- **Capacidade** `settings`: só o que as telas de ajustes usam (configurações, widgets, apps, pacotes de ícones, fechar a própria janela), pinada no guarda.
- **O painel** continua com as telas de ajustes por enquanto, mas ninguém mais as abre; elas saem dele no H2.

## 4. Verificado (H1)

No app real (debug): o item Configurações abre a janela "Configurações do DockMax" com as cinco seções e Geral escolhida; a seção Geral cabe sem nada cortado (captura); Esc fecha a janela e a página some (a janela é destruída).

**Não verificado:** o clique de mouse físico no item; a bandeja; a suíte de ponta a ponta local falhou inteira antes de qualquer passo (inclusive os testes de Dock e Nota, que esta mudança não toca: o app de teste não respondeu ao WebDriver), o que aponta para o ambiente local; os testes de ponta a ponta foram ajustados para a janela nova e a CI deve rodá-los.

## 5. H2 em detalhe

- **A janela do painel** (`windows/panel_window.rs`) saiu do `tauri.conf.json`: é criada (fora da thread principal, como a de Configurações) quando um painel vai abrir, ou quando o ponteiro **para 150 ms** sobre um item que abre painel (aquecer), e é **destruída 45 s** depois do último uso, sem painel aberto nem esperando. Com ela, o processo do WebView2 encerra.
- **O primeiro show espera a página:** um painel pedido com a janela ainda nascendo fica guardado e sai quando a página avisa que escuta o `panel-show` (`panel_mounted`).
- **O que não aquece:** os programas (o clique abre o programa) e Configurações (um app à parte).

## 6. H3 em detalhe

- **A janela** (`windows/native_dock/`): uma janela em camadas, sempre no topo, que nunca é ativada pelo clique (o programa em uso continua na frente), desenhada com **Direct2D** num bitmap e entregue por `UpdateLayeredWindow`. Onde é transparente o clique atravessa; uma barra sem fundo (Flutuante, Prateleira Aqua) leva um quase nada de alfa para segurar o mouse entre os ladrilhos. Tudo é desenhado em pixels lógicos, no DPI do monitor.
- **Regras puras em Rust, com os testes do TypeScript:** a geometria (`domain/dock_layout.rs`, de `layout.ts`, `magnification.ts` e `drag.ts`), a lista de itens (`domain/dock_items.rs`) e o que os ladrilhos vivos mostram (`domain/live_faces.rs`, de `formatDockTime` e `progressOf`).
- **O que o dock desenha:** os itens em grupos com separador (fixados, widgets, abertos, sistema), o esmalte de cada ladrilho (a superelipse, os dois tons, o brilho), os ícones dos programas (do pacote de ícones ou do Windows, lidos fora da thread principal), a foto do perfil do Chrome, a primeira letra enquanto não há ícone, os glifos de Configurações, Sobre, Bandeja e Nota, o relógio (do fuso dele, 24 h ou 12 h, digital ou analógico), o anel do Progresso do tempo, o ponto (ou traço) de aberto, o aviso de atenção em Configurações, o rótulo, o relógio e a data no fim da barra, os **nove estilos** com borda, cantos, brilho, contorno do Neon, prateleira da Aqua e **reflexos**.
- **O que ele faz:** ampliação animada (os três efeitos), clique (abre, ativa, minimiza, passa de janela; painel ancorado no item), botão do meio (nova instância), botão direito (menu dos programas), arrastar um fixado para reordenar (o ícone sobe um pouco e segue o ponteiro, os outros deslizam e abrem espaço onde ele cairia, e ao soltar ele desliza até o lugar novo; sem animação quando o Windows pede menos movimento), soltar arquivos para fixar, a animação de abertura (pular, pulsar, tremer), o calendário no relógio do fim, e o aquecimento do H2.
- **Depois do primeiro uso (decisão do autor, 2026-10-26):** Configurações e Sobre saíram dos itens do dock. Configurações abre pelo **menu do botão direito** do dock (fora de um programa) e tem o Sobre dentro. O item de Configurações volta só enquanto carrega o aviso de atenção do banco. A Bandeja ganhou a seta para cima da barra do Windows. O arrasto vale para os programas abertos (até o fim da sessão) e mistura os **apps fixados e os widgets** num grupo só: cada tipo guarda a própria ordem, e a ordem conjunta (`meta` "dock_order" no banco) diz só onde cada tipo entra, então reordenar em Configurações também vale no dock.
- **A bandeja como a do PrimoDock:** com a barra do Windows oculta, o fim do dock tem uma faixa de ícones pequenos em duas linhas, antes do relógio, desenhados com a fonte de ícones do próprio Windows (Segoe Fluent Icons): a **seta** (só com a barra oculta por completo; abre o painel da bandeja), os **ícones da bandeja escolhidos** (até 6, em Configurações › Geral, "Painel · Dock", guardados pelo nome `path:<programa>#<número>`, que sobrevive a reinícios; o painel da seta não os repete), **rede, volume e bateria** (o estado de verdade, lido a cada 3 s; sem bateria, sem ícone). Clicar em rede, volume ou bateria abre as configurações rápidas do Windows; um ícone da bandeja recebe o clique como na barra.
- **Acessibilidade:** um provedor de **UI Automation**: uma barra de ferramentas "Dock" com um botão por item (nome, com o aviso em Configurações; posição; **Invoke**, que age como o clique). O dock não recebe o teclado: ele nunca é ativado.
- **O que saiu:** o dock em React (`src/dock/`, menos `look.ts`, que a Aparência usa), a página `index.html`, o rastreador do mouse (`hit_test`), a capacidade `dock` e os comandos que só ele chamava (`toggle_panel`, `warm_panel`, `set_dock_layout`, `open_flyout`). A janela do painel não avisa mais a página do dock por evento: chama o dock.
- **Detalhes que mudaram com o dock nativo:** o Tauri encerrava o app quando a última janela dele fechava; com o dock fora do Tauri, a saída sem código é impedida (o "Sair" da bandeja sai). O COM/OLE da thread principal, que o WebView2 inicializava, agora é do dock.
- **Ponta a ponta:** o build de teste ganha uma janela oculta, `e2e`, com dois comandos só dele: `e2e_dock_view` (os itens, a face de cada um, se o dock está escuro) e `e2e_press_dock` (aperta um item pelo nome).

## 7. Verificado (H2 e H3)

No app real (release, máquina do autor, um monitor, escala de 125 %), com os itens do autor (12 widgets e programas, Configurações, Sobre, Bandeja):

| Medida | Resultado |
|---|---|
| Processos em repouso | **1** (nenhum WebView2) |
| Memória privada (Private Bytes) em repouso | **64,5 MB** (antes ~165 MB em 6 processos; o spike deu ~60 MB) |
| Working set em repouso | 70 MB (privado: 17,7 MB) |
| CPU em repouso | 0 % |
| Com o Sobre aberto | 6 processos, 85 MB de working set privado no total |
| 52 s depois de fechar o painel | a janela do painel liberada, o WebView2 encerrado, o app de pé; 1 processo |
| Abrir o Sobre a frio (sem WebView2) | **393 ms** até o painel na tela |
| Abrir de novo depois de liberado | 336 ms |
| Abrir com a janela mantida (morno) | **15 ms** |

Também verificado: o dock desenha os itens do autor (relógios em dois fusos, anéis de progresso, nota, programas com o ponto de aberto e a foto do perfil do Chrome, os itens do sistema, o relógio e a data no fim, captura); a UI Automation lista a barra "Dock" com os 13 botões, os nomes e as posições na tela, e o Invoke em "Sobre" abre e fecha o painel ancorado no item.

**Não verificado:** a ampliação com o mouse de verdade (a mensagem de mouse simulada é desfeita na hora pelo "saiu", porque o cursor real não está lá); arrastar para reordenar; soltar arquivos; os botões do meio e direito; o Narrador; os estilos além do Nativo e os reflexos na tela; a borda de cima; escala de 150 %; a suíte de ponta a ponta (a CI roda).
