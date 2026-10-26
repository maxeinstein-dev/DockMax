# Spec 0010 — Fase D4: tela cheia e um dock por monitor

Continuação da [spec 0009](0009-fase-d-janelas.md). Requisitos do autor (2026-10-26):

1. **Tela cheia por monitor.** O PrimoDock tem um defeito: com um dock em cada monitor e um vídeo em tela cheia num deles, usar algo no outro monitor faz o dock reaparecer **por cima do vídeo**. No DockMax, cada dock decide se some olhando **só o seu monitor**, nunca o programa em primeiro plano.
2. **O dock ocupa o lugar da barra de tarefas** em cada monitor: reserva a faixa da borda, e os programas ficam acima dele, não por baixo.
3. **Economia de memória.** A primeira comparação foi o PrimoDock completo contra o DockMax daquele momento; se um dock a mais custar caro, a arquitetura deve ser revista antes.

## 1. Recortes

| Recorte | Entrega |
|---|---|
| **D4a** | O dock some quando o seu monitor está em tela cheia e volta quando ela acaba (ainda um dock, no monitor principal) |
| **Memória** | Antes do D4b: medir onde vai a memória (por processo do WebView2) e o que cada alternativa de arquitetura economiza; decidir por ADR |
| **D4b** | Um dock em cada monitor, cada um com a sua faixa reservada e a regra de tela cheia do seu monitor |

## 2. D4a em detalhe

- **A regra** (`domain/fullscreen.rs`): entre as janelas que teriam botão na barra (visíveis, não minimizadas, não encobertas, não janelas de ferramenta, nem a área de trabalho nem as barras do Windows, nem as do próprio DockMax), em ordem de empilhamento, a **mais alta que está no monitor** do dock; se ela cobre o monitor inteiro, o monitor está em tela cheia.
  - Usar o outro monitor (o primeiro plano vai para lá) não muda a janela mais alta deste: o dock continua escondido. É o requisito 1.
  - Uma janela maximizada para acima da faixa do dock, então não é tela cheia.
  - Uma janela normal posta por cima do vídeo no mesmo monitor faz o dock voltar: o usuário está trabalhando ali.
- **Quando olhar:** o vigia de janelas da spec 0009 passa a ouvir também a mudança de posição e tamanho (`EVENT_OBJECT_LOCATIONCHANGE`, que é o que acontece quando um vídeo entra ou sai de tela cheia), com uma marca própria: uma mudança de posição só refaz a checagem de tela cheia (barata, sem COM), não a lista de programas abertos.
- **Esconder e voltar:** o dock some como no "Ocultar" da bandeja (o painel aberto fecha) e volta sozinho quando a tela cheia acaba. Se o usuário mostrar ou esconder o dock nesse meio tempo, a escolha dele vale e o dock não volta sozinho. A faixa reservada continua reservada (a tela cheia cobre tudo de qualquer jeito).

## 3. Memória: o ponto de partida

| Medição (memória privada, processo e filhos) | Resultado |
|---|---|
| PrimoDock 0.7, **dois** docks, 3 espaços, 2 widgets, ~15 apps ([referência](../reference/primodock/instalacao.md)) | 179 MB |
| DockMax, um dock, 5 programas abertos mostrados (spec 0009) | ~200 MB |
| DockMax, spike com um **segundo** dock (página do dock numa segunda janela, no outro monitor) | 220–230 MB |

O segundo dock custa uns 15–25 MB, e não um processo a mais: com `--process-per-site` as páginas dividem o renderizador. Mesmo assim o DockMax já pesa mais que o PrimoDock fazendo menos.

**Decidido** ([ADR 0010](../adr/0010-gpu-no-processo-do-navegador.md), com a medição por processo e as alternativas): a GPU do WebView2 passa para o processo do navegador, com reinício automático se ele cair. Um dock fica em ~165 MB e dois em ~196 MB. O próximo corte é o **app de Configurações**, aberto só quando usado, antes do D4b.

## 4. Verificado (D4a)

No app real (release), com dois monitores: uma janela de teste em tela cheia no monitor principal esconde o dock; trazer para a frente uma janela no **outro** monitor (o primeiro plano foi para lá) **mantém o dock escondido**; fechar a tela cheia traz o dock de volta.

**Não verificado:** um vídeo de verdade (YouTube, player) em tela cheia, jogos em tela cheia exclusiva e o dock escondido pelo usuário durante a tela cheia.
