# ADR 0011 — Spike: o dock desenhado em Rust nativo, sem WebView2

**Status:** proposto (2026-10-26). A decisão é do autor; este ADR registra a medição.

## Contexto

Depois do [ADR 0010](0010-gpu-no-processo-do-navegador.md) o DockMax está em ~165 MB com um dock (perto dos 179 MB do PrimoDock com dois). O autor perguntou por que não copiar a arquitetura do PrimoDock (.NET/WPF): ela não é mais leve que a nossa; o que de fato economizaria seria desenhar o dock **sem** um navegador embutido. Este spike mede isso antes de qualquer decisão.

## O spike

`spikes/native-dock/` (fora do build do app): uma janela em camadas, sempre no topo, que deixa passar o clique onde é transparente, desenhada com **Direct2D** num bitmap e entregue ao Windows por `UpdateLayeredWindow`. Tem a barra arredondada translúcida, os ícones reais de 14 programas (do shell, via WIC), um relógio com **DirectWrite** e a ampliação ao passar o mouse animada a ~60 fps. `--animate` anima sozinho, para medir sem mexer no mouse.

## Medição (release, monitor principal, mesma máquina)

| | DockMax hoje (WebView2, ADR 0010) | Dock nativo (spike) |
|---|---|---|
| Memória privada em repouso | ~165 MB, 6 processos | **~60 MB, 1 processo** (working set 54 MB) |
| CPU em repouso | 0,06–0,16 % | 0 % |
| CPU animando a ~60 fps | — | 0,19 % |

Boa parte dos 60 MB são as bibliotecas do Direct2D, do DirectWrite e do WIC e os ícones já decodificados; um dock nativo completo ficaria um pouco acima.

## O que a medição não diz

- **O custo de escrever.** O dock de hoje (React/CSS) tem nove estilos de barra, ampliação, rótulos, indicador, arrastar, faces vivas dos widgets (relógio, progresso, nota), tela cheia, menus. Tudo isso teria de ser redesenhado em Direct2D. O lado Rust (janelas, apps abertos, Jump Lists, bandeja, app bar, barra do Windows) já está pronto e seria reaproveitado.
- **Painéis e Configurações.** O desenho natural é **híbrido**: o dock nativo o dia inteiro e janelas WebView2 só quando um painel ou as Configurações abrem, fechadas de verdade depois (o processo do WebView2 encerra quando não há nenhuma). Em repouso ficaria perto do número nativo; com um painel aberto, sobe enquanto ele estiver aberto.

## Opções

1. **Manter o dock em WebView2** (hoje, ~165 MB; ~196 MB com dois docks) e seguir com o app de Configurações e o D4b.
2. **Híbrido:** dock nativo + painéis e Configurações em WebView2 sob demanda. Em repouso perto de 60–80 MB e ~15 MB ou menos por monitor a mais. Custo: reescrever a camada visual do dock; os painéis em React continuam.

O app de Configurações em janela própria e sob demanda (decidido) serve às duas opções.
