# Spec 0017 — Fase F: widgets

Fase F da [spec 0004](0004-paridade-primodock.md) (§3.5), depois da fase E. Referência do PrimoDock em [`docs/reference/primodock/interface.md`](../reference/primodock/interface.md) (aba Widgets: catálogo, "Em uso", largura do tile) e [`settings-e-dados.md`](../reference/primodock/settings-e-dados.md).

**Status:** em andamento: F1 (PR #52), F1b (#53), F2a-1 (Contas Google, #54 e #55, [ADR 0012](../adr/0012-contas-google.md)), F2a-2 (Calendário, #57), F2a-3 (Tasks, #58) e F2a-4 (cartão, #59) entregues; F2b-1 (o widget Ciclo, com o [guia do Atalho do iPhone](../guia-atalho-ciclo.md), #60), F2b-2 (avisos e lembretes, #61) e F2b-3 (o aviso no dock, #62) entregues; F2c-1 (Contagem regressiva) em curso; F2c-2 (Pomodoro) a F5 a fazer; o iCal ficou para depois.

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
| **F1b. Tamanho no dock** | Um item "Tamanho" no clique direito do widget (e nos Ajustes), só para os tipos que mostram algo a mais: Progresso do tempo, Atividade e Rede de 1 a 3 ícones (§6c). Relógio e Nota ficam com 1. |
| **F2a. Calendário** | As agendas Google (várias contas) e por endereço iCal, com as tasks do Google Tasks; ver §6c. |
| **F2b. Ciclo** | O ciclo da noiva do autor, lido da agenda dela; ver §6c. |
| **F2c. Tempo** | **Contagem regressiva** (dias até uma data, com nome) e **Pomodoro / timer de foco** (foco e pausa, com o tempo restante no ícone; o estado fica em Rust, então continua contando com o painel fechado; aviso do Windows no fim). |
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

## 6b. Como o F1 ficou (2026-10-27)

- **Tile largo:** o dock nativo soma larguras (`dock_layout::span_width`); um tile de 2 ícones é um retângulo arredondado com o mesmo esmalte (`paint.rs`, `wide_glaze`).
- **Atividade** (`activity`): mostra os dois, só o processador ou só a memória; no dock, em barras, anéis ou números; 1 ou 2 ícones. Esmalte Vinho (`.t-activity`).
- **Atividade de rede** (`network`): 1 ícone com as duas taxas curtas (`↓340K`), 2 ícones (o padrão) com as taxas e o gráfico do último minuto. Esmalte Índigo (`.t-network`).
- **Provedor:** um amostrador único (`windows/system_activity.rs`) lê CPU, memória e rede. O dock liga um timer de 2 s só enquanto um desses widgets está nele; sem eles, nada é lido. Leituras com menos de 1,5 s de diferença reaproveitam a última, então dois monitores e o painel leem o sistema uma vez.
- **Painel:** em vez do evento `widget-data` da §3, o painel aberto pede a leitura a cada 2 s (`read_system_activity`, só na janela do painel) e para ao fechar.
- **Aba Widgets:** substitui "Itens do dock" em Configurações (o id da seção continua `items`). Em cima, "Em uso" (a lista do espaço, com subir, descer, remover e os widgets de outros espaços); embaixo, o catálogo, com busca sem acentos e prateleiras Sistema, Tempo e Produtividade. **Largura e estilo** se ajustam nos Ajustes de cada widget (pelo painel), não na linha de "Em uso"; levar isso para a linha fica para depois, se fizer falta.

## 6c. Decisões do grilling de 2026-10-28

**Tamanho no dock (F1b).** Como no PrimoDock, o tamanho se troca no próprio dock, pelo clique direito ("Tamanho"), e também nos Ajustes. Só os tipos que ganham algo:

| Tipo | 1 ícone | 2 ícones | 3 ícones |
|---|---|---|---|
| Progresso do tempo | o anel | dois anéis (os dois primeiros períodos) | três barras com o nome do período |
| Atividade | barras, anéis ou números | como no F1 | processador e memória com o gráfico do último minuto |
| Atividade de rede | as taxas curtas | taxas e gráfico | o gráfico maior com as taxas por extenso |
| Calendário (F2a) | quando é a próxima reunião | quando e o título; **com o mouse em cima, levita** num andar acima do dock, da largura de 3 ícones, por cima dos vizinhos (que não se mexem), com "15:00 Alinhamento" e "Depois 20:00 Jiu Jitsu" | — (pequeno demais para ler, decisão do autor de 2026-10-28) |

Relógio e Nota ficam com 1 ícone.

**Calendário (F2a).**
- **Fontes:** contas Google, quantas o autor conectar, como no Google Agenda: as dele, a da Alfama e a da noiva, que ele já acessa e que entra inteira, para ver se os compromissos dela batem com os dele. Também agendas assinadas por endereço iCal. A Microsoft fica para quando fizer falta. Cada agenda tem a sua cor e se liga e desliga nos ajustes. **iCal** (decisão do autor, 2026-10-28): fica para depois. A conta da Alfama conectou pelo Google, então o iCal só volta se aparecer uma agenda fora do Google. Ele exige ler reuniões que se repetem (RRULE) e os fusos no formato do Outlook.
- **Login:** OAuth do Google com `calendar.readonly` (as agendas e a lista delas), `calendar.events` (criar reuniões) e `tasks` (concluir tarefas). A escrita é pedida desde o começo (decisão do autor, 2026-10-28), para que a tela de criar reunião, quando chegar, não obrigue a reconectar as contas. O projeto no Google Cloud fica na conta pessoal do autor, em produção e sem verificação: uma tela de "app não verificado" no primeiro login, e o login não cai a cada 7 dias. Os tokens ficam no cofre de credenciais do Windows. Se a Alfama bloquear apps de fora, essa conta entra pelo endereço iCal.
- **Tasks** (F2a-3; decisão do autor, 2026-10-28): do Google Tasks, as do dia e as atrasadas, de todas as listas das contas, no painel do Calendário. Cada uma tem uma caixa para **marcar como feita**, que salva no Google na hora. Só as tarefas mostradas podem ser concluídas, e uma recusa do Google traz a tarefa de volta com aviso. Os ajustes do widget têm "Tarefas do Google: mostrar/ocultar". **Criar reunião** pelo widget vem depois, sempre com confirmação e só nas agendas do próprio autor. O Notion (talvez como widget próprio) fica para depois.
- **Painel:** hoje e amanhã, com os eventos de dia inteiro numa faixa no topo e os recusados escondidos. "Entrar" na linha da reunião (Meet, Teams, Zoom). Clicar num evento abre os detalhes no próprio painel: hora, local, descrição, participantes, Entrar e "Abrir no Google Agenda". O clique do meio no tile entra na próxima reunião, se ela começa em até 10 min.
- **Atualização:** a cada 1 minuto, e na hora ao abrir o painel, a partir de uma thread própria, só enquanto houver um Calendário no dock. Cada leitura pede de novo os dois dias de cada agenda (`timeMin`/`timeMax`, `singleEvents`). O `syncToken` incremental pensado no grilling não pode ser combinado com essa janela de tempo e a ordenação, e as respostas dos dois dias são pequenas. A lista de agendas é relida a cada 30 minutos. O push do Google exige um servidor público, então fica fora. Os eventos ficam só na memória, de nenhuma agenda vai nada para o disco; sem internet, o painel diz "sem conexão" e mostra a última leitura.
- Os eventos do ciclo ("🌸") ficam **escondidos** no Calendário.

**Cartão do Calendário (F2a-4; decisões do autor, 2026-10-28).** Com o mouse em cima, o Calendário levita (F2a-3) e mostra mais num **cartão próprio**, mais alto e mais largo que o tile, numa janela própria como o cartão das prévias da fase E, porque a janela do dock não tem altura para ele.
- **Conteúdo:** reuniões e tarefas por padrão; os ajustes do widget escolhem só reuniões ou reuniões e tarefas.
- **Período:** editável nos ajustes. O cartão rola entre as opções "o dia", "o dia e amanhã", "3 dias" e "a semana". O padrão é o dia; depois das 18h, as de amanhã até as 12h.
- **Clique:** clicar numa reunião do cartão abre os detalhes no painel. Para entrar direto, vale o clique do meio no tile, como hoje.

**Contagem regressiva (F2c-1).** O nome é o Rótulo do widget ("Casamento"); a config tem só a data e "Repetir todo ano" (aniversários: conta até o próximo; 29 de fevereiro cai no dia 28 nos anos sem ele). No dock, os dias que faltam e "dias" embaixo; "Hoje" no dia; depois, os dias que passaram com "dias atrás". Sem data, um traço. Esmalte Violeta (`.t-countdown`), longe dos tons da Nota e da Rede.

**Pomodoro (F2c-2).** Como no Ciclo, o fim de cada etapa aparece no dock (balanço e selo), e a notificação do Windows é opção desligada: o autor não quer depender dela.

**Ciclo (F2b).** É um widget próprio, que pode ficar num espaço de trabalho fora do que o autor compartilha em reunião.
- **Origem:** a noiva controla tudo pela agenda dela. Ela marca o primeiro dia com um evento de dia inteiro "🌸" (o símbolo se troca nos ajustes), por um Atalho de um toque no iPhone ou pela Siri. O DockMax lê esses eventos dos últimos 12 meses e não guarda nada em disco. No disco ficam só os ajustes do autor.
- **Previsão:** o ciclo é a média dos últimos 6 (28 dias com menos de 2 registros). A menstruação dura a média dos eventos dela (5 dias sem histórico); ela corrige esticando ou encurtando o evento. Não há janela fértil nem ovulação.
- **Discrição:** o tile não usa as palavras "menstruação" ou "TPM"; mostra só um ícone neutro e uma cor ou contagem. Os detalhes ficam no painel.
- **Avisos editáveis:** a TPM começa N dias antes (5 por padrão; o autor ajusta para os meses seguintes), a véspera e o dia previsto, mais lembretes livres do autor (um texto, N dias antes ou depois de uma fase). O aviso aparece no próprio dock (F2b-3): a partir das 9h o bloco do Ciclo ganha um selo e balança como um app abrindo; o selo some quando o painel abre, e o que é o aviso só aparece lá, na seção "Hoje". O autor não quer depender das notificações do Windows: a notificação de texto neutro ("Lembrete do Ciclo") é uma opção, desligada por padrão. Não há aviso de atraso.
- O passo a passo do Atalho do iPhone vai junto do F2b.

## 7. Verificação

Por recorte: os testes das regras puras (larguras, leituras do sistema com números de mentira, as contas do calendário e do pomodoro), as fixtures de contrato, o teste ao vivo com a cópia de teste (o widget no dock, o painel, a troca de largura) e a memória com e sem os widgets novos.
