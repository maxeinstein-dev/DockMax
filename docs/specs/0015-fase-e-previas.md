# Spec 0015 — Fase E: prévias das janelas

Fase E da [spec 0004](0004-paridade-primodock.md) (§3.7a e §5). A bandeja dessa fase já está pronta (painel da seta, ícones no dock, menus no lugar); falta a prévia das janelas. Referência do PrimoDock em [`docs/reference/primodock/interface.md`](../reference/primodock/interface.md) (prévia de janela e a seção "Prévias de janela" dos ajustes) e [`settings-e-dados.md`](../reference/primodock/settings-e-dados.md) (`showsPreviews`, `previewDelay`, `previewsOnlyWithSeveralWindows`).

## 1. O que é

Parar o ponteiro sobre um programa aberto no dock mostra, depois de um atraso curto, um **cartão acima do ícone** com uma miniatura ao vivo de cada janela dele, o título e um botão de fechar. Como a barra do Windows e o PrimoDock.

- **As janelas** são as do programa no espaço de trabalho daquele dock (spec 0013 C2), na ordem em que o DockMax as encontrou (a mesma dos outros cálculos dele, não a da frente primeiro).
- **Cada janela** ocupa uma célula: o título no alto (uma linha, com reticências), o botão de fechar à direita dele e a miniatura embaixo, ao vivo, na proporção da janela.
- **Clicar numa célula** traz aquela janela para a frente (restaura se estiver minimizada) e fecha o cartão. **Clicar no X** pede à janela que feche, como o "Fechar janela" do menu; o cartão continua com as outras e some quando não sobra nenhuma.
- **O cartão some** quando o ponteiro sai do ícone e do cartão (com uma tolerância curta para atravessar o vão), num clique no ícone, ao começar um arrasto, ao abrir um menu ou um painel, e quando o dock se recolhe (ocultar automaticamente).

## 2. Configurações

No grupo **Comportamento** (Configurações › Geral), como no PrimoDock:

| Opção | Chave | Padrão |
|---|---|---|
| Prévia ao passar o mouse | `dock.previews` | ligada |
| Atraso da prévia | `dock.previewDelay` | 200 ms (0 a 1000) |
| Só com 2 ou mais janelas | `dock.previewsOnlySeveral` | desligada |

O atraso e "só com 2 ou mais" aparecem só com a prévia ligada (como os tempos do ocultar automaticamente).

## 3. Como é feito

- **A miniatura ao vivo é do DWM** (`DwmRegisterThumbnail`): o Windows desenha a janela de origem dentro do cartão, sem copiar pixels para o DockMax. Uma miniatura por janela, desregistrada quando o cartão some.
- **O cartão é uma janela própria do Windows**, não a do dock: o dock é uma janela em camadas desenhada num bitmap (`UpdateLayeredWindow`, spec 0011), e a miniatura do DWM precisa de uma janela comum de destino. Janela popup sem foco (`WS_EX_NOACTIVATE`), fora do Alt+Tab (`WS_EX_TOOLWINDOW`), sempre no topo, com os cantos arredondados do Windows 11 e sombra. O fundo, os títulos e o X são desenhados com Direct2D nas cores do tema do dock (claro ou escuro).
- **Uma janela só**, criada na primeira prévia e reaproveitada (memória: spec 0011, medir no PR).
- **A geometria é pura** (`domain/previews.rs`): dado o ícone, a borda do dock, a área de trabalho do monitor e o tamanho de cada janela, onde fica o cartão e cada célula. Miniatura de até 200 × 120 px lógicos; com muitas janelas, as células encolhem até caber na largura do monitor (no mínimo 96 px), e o que ainda não couber fica de fora com "+N" no fim. O cartão nunca sai da tela.
- **Janela minimizada:** o DWM mostra a última imagem dela; se não houver nenhuma, a célula mostra só o título.

## 4. Recortes

| Recorte | Entrega |
|---|---|
| **E1. O cartão** | Prévia ao passar o mouse com miniaturas ao vivo, título, ativar e fechar; as três opções em Configurações. |
| **E2. Acabamento** | O cartão pelo leitor de tela e a entrada animada. |

Cada recorte é um PR, com o plano, a verificação no app real e a memória medida.

## 4b. E2 em detalhe (feito)

- **Leitor de tela:** o cartão é uma barra de ferramentas "Janelas de <app>", servida pelo mesmo provedor de UI Automation do dock (`native_dock::access`, que agora atende outra janela com o nome e o jeito de apertar dela). Cada janela é um botão com o título dela ("Janela sem título" sem título), seguido de "Fechar <título>". Acionar um botão faz o mesmo que o clique; o pedido chega da thread da UI Automation e é feito na thread do cartão.
- **Entrada animada:** um cartão novo entra deslizando 8 px do lado do dock em 120 ms, desacelerando no fim. Um cartão que toma o lugar de outro (o ponteiro em outro programa, uma janela fechada) só aparece. Sem animação quando o Windows pede menos movimento. A saída continua imediata, como a da barra do Windows.
- **Decisão do autor (2026-10-27): sem o "Espiar".** Esmaecer as outras janelas da tela, como a barra do Windows faz, só é possível com uma função não documentada do DWM (`DwmpActivateLivePreview`), que pode mudar numa atualização do Windows. A célula sob o ponteiro continua realçada no próprio cartão.
- **Teclado:** fica de fora junto com o do dock. O dock nativo nunca toma o foco (spec 0011), então o cartão não tem como receber as setas; quem usa o teclado chega às janelas pelo Alt+Tab, e o leitor de tela chega ao cartão pelos botões acima.

## 5. Fora desta fase

O "Espiar" (decisão do autor, §4b), o cartão pelo teclado (§4b), controles de mídia e botões da barra de miniaturas (`ITaskbarList3::ThumbBarAddButtons`), prévia das abas de um navegador, e a prévia sobre os widgets.

## 6. Perguntas em aberto para o autor

Valem as propostas até o autor decidir:

1. Com a prévia aberta, passar para outro programa aberto troca o cartão na hora, sem esperar o atraso (proposta, como a barra do Windows)?
2. Um programa fixado sem janelas não mostra cartão nenhum (proposta), ou mostra o nome dele?

## 7. Verificação

- **Verificado (E1, 2026-10-27),** testes da geometria e das opções, e no app real (release, os dados do autor, o ponteiro de verdade, duas janelas de teste de um mesmo programa):
  - parado sobre o ícone, o cartão apareceu acima dele com as duas miniaturas ao vivo, os títulos e os X (422 × 164 px a 100 %);
  - atravessar do ícone para o cartão não o fechou; longe, ele sumiu;
  - o clique numa miniatura levou à janela dela e fechou o cartão;
  - o X sob o ponteiro ficou vermelho; o clique fechou aquela janela, e o cartão continuou só com a outra;
  - memória privada: 84,2 MB ao abrir, 87,3 MB depois de usar o cartão.
- **Não verificado (E1):** trazer para a frente uma janela que estava atrás (a clicada já era a da frente), uma janela minimizada, o segundo monitor e o dock no topo.
- **Verificado (E2, 2026-10-27):** testes do deslize (do lado do dock, desacelerando, parado no fim) e do código de cada botão do leitor de tela.
- **Testes:** a geometria (cartão acima, abaixo e ao lado conforme a borda; dentro da tela; células encolhendo; "+N"), as opções no arquivo e no comando de configurações, e quando a prévia abre (atraso, "só com 2 ou mais", sem janelas no espaço).
- **App real:** passar o mouse num programa com uma e com várias janelas, nos dois monitores e com o dock em cada borda; clicar numa miniatura (inclusive minimizada); fechar pelo X; sair do ícone para o cartão sem ele sumir; medir a memória antes e depois.
