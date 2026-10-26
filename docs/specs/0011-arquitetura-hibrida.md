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
- **Quem abre:** o item Configurações do dock (o `toggle_panel` com `settings` fecha o painel aberto e abre o app) e "Configurações" no menu da bandeja.
- **A página** (`settings.html`, `src/settings/`): barra lateral com Geral, Aparência do dock, Apps do dock, Itens do dock e Sobre, e ao lado a mesma tela que o painel tinha, sem o "Voltar". Esc fecha a janela.
- **Capacidade** `settings`: só o que as telas de ajustes usam (configurações, widgets, apps, pacotes de ícones, fechar a própria janela), pinada no guarda.
- **O painel** continua com as telas de ajustes por enquanto, mas ninguém mais as abre; elas saem dele no H2.

## 4. Verificado (H1)

No app real (debug): o item Configurações abre a janela "Configurações do DockMax" com as cinco seções e Geral escolhida; a seção Geral cabe sem nada cortado (captura); Esc fecha a janela e a página some (a janela é destruída).

**Não verificado:** o clique de mouse físico no item; a bandeja; a suíte de ponta a ponta local falhou inteira antes de qualquer passo (inclusive os testes de Dock e Nota, que esta mudança não toca: o app de teste não respondeu ao WebDriver), o que aponta para o ambiente local; os testes de ponta a ponta foram ajustados para a janela nova e a CI deve rodá-los.
