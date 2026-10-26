# ADR 0010 — GPU do WebView2 no processo do navegador, com reinício automático

**Status:** aceito (2026-10-26, decisão do autor).
**Substitui:** o descarte de `--in-process-gpu` da [spec 0003 §2.12](../specs/0003-fase-2-widgets-e-persistencia.md).

## Contexto

O DockMax substitui a barra de tarefas e vai ter um dock por monitor (fase D4, [spec 0010](../specs/0010-fase-d4-monitores-e-tela-cheia.md)). A primeira comparação de memória foi o PrimoDock completo contra o DockMax daquele momento; hoje o DockMax pesa mais fazendo menos, e o autor pediu que a arquitetura fosse revista para economizar.

Medição de 2026-10-26 (build de release, memória privada do processo e de todos os filhos, 30 s depois de abrir, máquina do autor com dois monitores):

| Configuração | Total | Processos | CPU em repouso |
|---|---|---|---|
| Hoje: GPU em processo próprio | 189–207 MB (GPU 77, renderizador 41, navegador 36, DockMax 14, rede 12, armazenamento 7, crashpad 3) | 7 | 0,06–0,38 % |
| `--in-process-gpu` | **165 MB** (navegador 86, renderizador 43, demais iguais) | 6 | 0,12–0,16 % |
| `--disable-gpu` | 127–131 MB | 7 | — |
| `--disable-gpu-compositing` | 172 MB | 7 | 0,62 % |
| `--renderer-process-limit=1` | 191 MB (nada muda: já é um renderizador) | 7 | — |
| Dois docks, GPU própria | 238 MB | 7 | — |
| Dois docks, `--in-process-gpu` | **196 MB** | 6 | — |
| PrimoDock 0.7 com dois docks ([referência](../reference/primodock/instalacao.md)) | 179 MB | — | — |

- `--disable-gpu`: o dock **não aparece** (a janela transparente não é mostrada). Descartado.
- `--disable-gpu-compositing`: desenha, mas a composição em software deixa a CPU em repouso em 0,6 %. Descartado.
- `--in-process-gpu`: o visual fica igual (capturas comparadas) e a CPU continua na faixa de ruído. Economiza 36–42 MB com um dock, e o segundo dock passa a custar ~14 MB em vez de ~37 MB.
- O alvo de memória baixo em repouso (`windows/memory.rs`) já estava ligado no dock e no painel.

O motivo do descarte na spec 0003 continua de pé: com a GPU dentro do processo do navegador, uma falha do driver de vídeo derruba o navegador, e com ele todas as janelas, em vez de só reiniciar o processo de GPU.

## Decisão

1. `--in-process-gpu` nos `additionalBrowserArgs` das janelas (iguais em todas, como pede o `--process-per-site`; um teste garante as duas opções).
2. **Reinício automático** (`windows/recovery.rs`): o DockMax assina o `ProcessFailed` do WebView2. Se o processo do navegador morre, ele devolve a bandeja, a faixa reservada e a barra do Windows (o mesmo caminho da saída normal), solta a trava de instância única e se reinicia. Se morrer de novo menos de 2 minutos depois do último reinício, ele fecha em vez de reiniciar, para não piscar para sempre com um driver quebrado; a barra do Windows volta do mesmo jeito.
3. O orçamento do roadmap passa a ser **cerca de 180 MB com um dock e cerca de 15 MB por monitor a mais**.

## Consequências

- Um dock: ~165 MB; dois docks: ~196 MB, perto dos 179 MB do PrimoDock com dois docks e mais coisas.
- Uma falha do driver de vídeo vira uma piscada do dock (verificado matando o processo do navegador: o DockMax voltou com um processo novo; numa segunda queda logo em seguida, ficou fechado; nos dois casos a barra do Windows foi devolvida).
- O próximo corte de memória é o **app de Configurações** (decisão do autor, 2026-10-26): uma janela criada só quando abre e fechada de verdade depois, tirando as telas de ajustes do painel que fica carregado o dia inteiro.
