# Spec 0017 — Fase F: widgets

Fase F da [spec 0004](0004-paridade-primodock.md) (§3.5), depois da fase E. Referência do PrimoDock em [`docs/reference/primodock/interface.md`](../reference/primodock/interface.md) (aba Widgets: catálogo, "Em uso", largura do tile) e [`settings-e-dados.md`](../reference/primodock/settings-e-dados.md).

**Status:** proposta.

## 1. Decisões do autor (2026-10-27)

| Pergunta | Decisão |
|---|---|
| Widgets de sistema | **Atividade** (CPU e memória) e **Atividade de rede**; a bateria fica de fora |
| Tempo, mídia e produtividade | **Tocando agora**, **Calendário**, **Pomodoro / timer de foco** e **Contagem regressiva** |
| Outros | **Clima**, **Lista de tarefas**, **Beber água** e **Uso do Claude** |
| Catálogo em Configurações | **No primeiro recorte**, junto dos primeiros widgets novos |

## 2. O que existe hoje

- Três tipos podem ser criados: Relógio, Progresso do tempo e Nota (`domain/widgets/`, `src/core/widgets.ts`). Atalho e Arquivos estão declarados, mas ainda não podem ser criados. O limite é de 12 widgets.
- **O ícone vivo é desenhado pelo dock nativo** (`native_dock/faces.rs`, `paint.rs`: `Face::Glyph`, `Time`, `Dial`, `Ring`). O painel de cada widget é uma página do WebView2 (`src/widgets/<tipo>/`, com Tile, Panel e Settings).
- **Todo widget é um quadrado**: a largura do item é a do ícone (`dock_layout.rs`), e não existe tile largo.
- **O único dado vivo é a hora** (o timer de minuto do dock e o `useNow` do painel). Não há leitura de CPU, memória, taxa de rede nem da mídia do Windows. O `system_status.rs` lê só bateria, tipo de rede e volume, para a faixa do fim do dock.
- Configurações › Itens do dock adiciona widgets por três botões fixos.

## 3. Base comum (vale para todos os recortes)

- **Largura do tile:** um widget pode ocupar de **1 a 4 ícones**. Cada tipo diz quais larguras aceita (a Atividade, por exemplo, aceita 1 e 2). A largura fica na config da instância; o layout do dock, o arrasto e o espaço reservado passam a somar larguras. Com o dock na vertical (futuro), um tile largo vira alto.
- **Dados vivos:** um provedor em Rust por fonte (sistema, mídia, clima), que só lê enquanto algum widget daquela fonte está no dock. Ele entrega um retrato ao dock nativo (para o ícone) e um evento `widget-data` ao painel aberto. A cadência é por fonte: CPU e rede a cada 2 s, mídia por eventos do Windows, clima a cada 30 min.
- **Cada tipo novo** segue o caminho que os três de hoje seguem: variante e config em Rust (com `validate` e default), as fixtures de contrato TS↔Rust, um desenho (`Face`) e um esmalte próprio (The Own Glaze Rule), nome em Rust e TS, e Tile, Panel e Settings no front.
- **Memória:** cada recorte mede o dock nativo com os widgets novos. Um provedor parado não pode custar nada.

## 4. Recortes

| Recorte | Entrega |
|---|---|
| **F1. Base, catálogo e sistema** | A largura do tile; o provedor de sistema; **Atividade** (CPU e memória, em barras, anéis ou números; 1 ou 2 ícones) e **Atividade de rede** (descida e subida; com gráfico em 2 ícones); a **aba Widgets** de Configurações (catálogo por categoria com miniatura, busca, adicionar ao espaço em uso; "Em uso" com largura, estilo e espaços), no lugar dos botões de Itens do dock. |
| **F2. Tempo** | **Calendário** (as próximas reuniões do Google Agenda, como o widget do celular; ver §6.5), **Contagem regressiva** (dias até uma data, com nome) e **Pomodoro / timer de foco** (foco e pausa, com o tempo restante no ícone; o estado fica em Rust, então continua contando com o painel fechado; aviso do Windows no fim). |
| **F3. Produtividade** | **Lista de tarefas** (tarefas com marca, guardadas no banco, o número de abertas no ícone) e **Beber água** (copos do dia, meta, lembrete opcional a cada N minutos, zera à meia-noite). |
| **F4. Mídia** | **Tocando agora**: a sessão de mídia do Windows (`GlobalSystemMediaTransportControlsSessionManager`), com capa, título e artista; tocar/pausar, anterior e próxima no painel e no ícone largo. |
| **F5. Fontes externas** | **Clima** (Open-Meteo, cidade digitada) e **Uso do Claude** (o Claude Code, pelos registros locais), como decidido na §6. |

Cada recorte é um PR, com o plano, a verificação no app real e a memória medida.

## 5. Fora desta fase

Pilhas de widgets (vários num lugar, trocados pela roda), espaçadores, "Widgets seguem o pacote de ícones", Bateria, Layout do teclado, World Clock, Uso do Codex e do OpenCode, Capturas de tela e Arquivos (que já está declarado e fica para depois).

## 6. Mais decisões do autor (2026-10-27)

1. **Clima:** o Open-Meteo (gratuito, sem chave nem conta), com a **cidade digitada** pelo usuário e sem localização automática. Só o nome da cidade (para achar as coordenadas) e as coordenadas vão para o serviço.
2. **Uso do Claude:** o uso do **Claude Code** lido dos registros locais (`~/.claude/projects/*.jsonl`: tokens e mensagens do dia e da semana), sem rede e sem credenciais. A conta pessoal não expõe o consumo do plano por uma API pública.
3. **Pomodoro:** 25 min de foco e 5 de pausa, com uma pausa longa de 15 min a cada 4 focos, e um aviso do Windows ao fim de cada etapa; os tempos mudam nos ajustes do widget.
4. **Catálogo:** uma aba **Widgets** em Configurações, no lugar de "Itens do dock", com o catálogo e o "Em uso"; a ordem dos widgets no dock continua em Espaços › Editar widgets.
5. **Calendário (2026-10-27):** ligado ao **Google Agenda** do autor, mostra as **próximas reuniões** e avança sozinho conforme o tempo passa, como o widget do celular. **Sem a data**, que o fim do dock já mostra. O ícone traz a próxima reunião (hora e começo do título, ou "em 10 min"); o painel lista as seguintes do dia e de amanhã. Como ligar à conta fica para o plano do F2: o **endereço secreto no formato iCal** (só leitura, sem login, colado nos ajustes; o endereço é um segredo e fica fora dos registros) ou o **OAuth do Google** (exige um projeto no Google Cloud e a tela de consentimento). A recomendação é começar pelo endereço iCal.

## 7. Verificação

Por recorte: os testes das regras puras (larguras, leituras do sistema com números de mentira, as contas do calendário e do pomodoro), as fixtures de contrato, o teste ao vivo com a cópia de teste (o widget no dock, o painel, a troca de largura) e a memória com e sem os widgets novos.
