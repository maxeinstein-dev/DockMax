# Spec 0013 — Fase C: espaços de trabalho

Fase C da [spec 0004](0004-paridade-primodock.md) (§3.3), a primeira depois da fase D. Referência do PrimoDock em [`docs/reference/primodock/`](../reference/primodock/README.md) (`workspaces.json`, `session.json`, aba Workspaces).

## Decisões do autor (2026-10-26)

1. **Trocar de espaço esconde as janelas dos outros espaços**, como o PrimoDock: cada espaço guarda as suas janelas, que somem quando se troca e voltam quando se volta.
2. **A troca vale por monitor**: cada monitor tem o seu espaço ativo, e trocar num não mexe no outro. Uma opção em Configurações, depois, deixa a troca valer para todos os monitores juntos.
3. **O DockMax começa com um espaço só.** Quem não usa espaços não vê nada novo; o seletor aparece com dois ou mais.
4. **Formas de trocar:** as bolinhas no dock, a roda do mouse sobre o dock, `Ctrl+Alt+1…9` e uma seção no menu do dock.

## 1. O que é um espaço

Um espaço de trabalho tem um **nome**, uma **cor** (uma paleta fixa, a mesma das bolinhas) e uma **lista de itens**: os programas fixados e os widgets que o dock mostra quando ele está ativo, na ordem do dock. Ele também **guarda janelas**: as que foram abertas enquanto ele estava ativo no monitor delas.

- O mesmo programa pode estar fixado em vários espaços; cada espaço tem a sua ordem.
- Um widget pode aparecer em vários espaços (como "Mostrar nos workspaces" do PrimoDock), mas é uma instância só: a nota é a mesma em todos.
- Os apps abertos não fixados, a bandeja e o relógio continuam no fim do dock; os apps abertos mostrados são só os das janelas do espaço ativo naquele monitor.

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **C1. Modelo e troca do dock** | Espaços no banco, migração que põe tudo o que existe num espaço "Principal", criar, renomear, mudar a cor, reordenar e excluir espaços em Configurações; fixar e desafixar valem para o espaço ativo do monitor; bolinhas, roda, `Ctrl+Alt+1…9` e menu trocam os **itens** do dock. As janelas ainda não mudam. |
| **C2. Janelas por espaço** | Cada janela tem um espaço; trocar esconde as dos outros espaços naquele monitor e mostra as do novo; o dock mostra só os apps abertos do espaço; tudo o que foi escondido volta se o DockMax fechar ou cair. |
| **C3. Acabamento** | "Mover para o espaço" no menu do programa; a cor do indicador seguindo o espaço (opção); a opção de troca global em vez de por monitor; widgets em vários espaços pela interface; as bolinhas para o leitor de tela; a troca animada. |

Cada recorte é um PR, com o plano e a verificação no app real, e mede a memória (o dock nativo não deve crescer de forma visível com espaços).

## 3. C1 em detalhe (feito)

### 3.1 Dados (migração 4)

A migração 3 ficou com o lançador (spec 0012); os espaços são a 4 (`src-tauri/migrations/0004_workspaces.sql`).

```sql
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY NOT NULL,      -- UUID v7
  name TEXT NOT NULL,
  color TEXT NOT NULL,               -- uma cor da paleta, #RRGGBB
  position INTEGER NOT NULL,         -- ordem das bolinhas e do atalho
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE workspace_items (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  item_key TEXT NOT NULL,            -- `app:<pinned_apps.id>` ou `widget:<widget_instances.id>`
  position INTEGER NOT NULL,         -- ordem no dock, apps e widgets misturados
  PRIMARY KEY (workspace_id, item_key)
) STRICT;
```

- **O primeiro espaço** ("Principal", azul) é criado pelo Rust na primeira leitura depois da migração, com os programas e widgets que existem, na ordem que o dock mostrava (`meta.dock_order`, que deixa de ser usado). Fazer isso em SQL puro exigiria refazer a regra de mistura do dock.
- **As chaves** são as que o dock já usava para a ordem. `pinned_apps` e `widget_instances` continuam sendo o catálogo: uma linha por programa e por widget. O que se repete é o item em cada espaço. Tirar um item de um espaço o remove só dali, e ele sai do catálogo (um widget com o texto da nota) quando nenhum espaço o tem mais.
- **Cada monitor** guarda o seu espaço em `meta` (`workspace:<nome do monitor>`); um monitor sem espaço guardado mostra o primeiro.
- **O espaço em uso** (`meta`, `workspace:in-use`) é o do dock usado por último: qualquer clique num dock o marca. É para onde vai o que nasce no dock sem dizer o espaço: um programa solto no dock (vai para o espaço daquele monitor), "Fixar no dock" e "Remover do dock" nos menus de um programa.
- **Excluir um espaço:** nunca o último. Os programas e widgets que só ele tinha saem para sempre, com uma confirmação que avisa disso. Os monitores que o mostravam passam para o primeiro.
- **Paleta:** oito cores fixas (as três do PrimoDock mais roxo, rosa, vermelho, amarelo e cinza). Até nove espaços.

### 3.2 Troca no dock

- **Bolinhas** no começo do dock, numa coluna, uma por espaço e na cor dele: a do espaço mostrado é cheia e maior, as outras são só o contorno. Aparecem com dois ou mais espaços. O clique troca; o nome aparece ao passar o mouse.
- **Roda do mouse** sobre o dock: o próximo para baixo, o anterior para cima, sem dar a volta.
- **`Ctrl+Alt+1…9`** (`RegisterHotKey`): vai ao espaço daquela posição no monitor onde está o ponteiro. Os atalhos só são registrados com dois ou mais espaços, e só os que existem: no teclado ABNT2, AltGr+1, 2 e 3 (¹ ² ³) chegam ao Windows como Ctrl+Alt e seriam tomados. Se outro programa já tiver um atalho, o DockMax segue sem ele e registra no log.
- **Menu do dock:** os espaços (com dois ou mais), cada um com o seu atalho e a marca no mostrado, e **"Novo espaço"**, que cria e mostra um espaço vazio naquele monitor.

### 3.3 Configurações

**Decisão do autor (2026-10-27):** como no PrimoDock, Configurações edita qualquer espaço, sem depender do que está na tela nem do espaço em uso ("posso estar usando o pessoal e querer alterar um app do de trabalho").

- **Seção "Espaços de trabalho":** os espaços como fichas (bolinha, nome, `Ctrl+Alt+N`) e "+ Novo espaço". A ficha escolhida abre o cartão do espaço: nome, cor, quantos apps e widgets, "Editar apps", "Editar widgets", subir, descer e excluir (com confirmação).
- **"Apps do dock" e "Itens do dock"** mostram e editam o espaço escolhido, com as fichas no topo para trocar (só com dois ou mais). Os comandos de apps e widgets recebem o espaço (`workspace`); sem ele, vale o espaço em uso.
- **Comandos novos**, só para a janela de Configurações: `list_workspaces`, `create_workspace`, `rename_workspace`, `recolor_workspace`, `move_workspace` e `delete_workspace`, com o evento `workspaces-changed`.

### 3.4 Ficou para depois

- ~~As bolinhas na UI Automation~~ e ~~a troca animada~~: feitas no C3.
- A aparência da seção, que entra no rebranding de Configurações e dos menus (roadmap).

## 4. C2 em detalhe: janelas por espaço (feito)

Como nas áreas de trabalho do Windows, mas **por monitor**: cada espaço guarda as suas janelas (`domain/window_spaces.rs`, `windows/window_spaces.rs`).

- **Dono:** uma janela de programa (as que a barra de tarefas mostraria, o mesmo critério dos apps abertos da spec 0009) pertence ao espaço mostrado no monitor dela quando apareceu. As que já estavam abertas quando o DockMax abriu também. **Uma janela levada a outro monitor passa a ser do espaço de lá** (proposta 1, valendo até o autor decidir).
- **Trocar** de A para B num monitor: as janelas daquele monitor que não são de B são **estacionadas** (`ShowWindow(SW_HIDE)`, o que as tira também do Alt+Tab); as de B estacionadas naquele monitor voltam como estavam (minimizadas continuam minimizadas), e a do topo vem para a frente. Os outros monitores não mudam.
- **O dock** mostra só os apps abertos do seu espaço, e o indicador de um programa fixado conta só as janelas dele ali. **Clicar num programa que só tem janelas em outro espaço abre uma janela nova** (proposta 2). Os menus de um programa ("Fechar janela") agem nas janelas do espaço daquele dock.
- **Excluir um espaço** mostra de volta as janelas estacionadas dele, que passam ao espaço do monitor delas.
- **Segurança, como a barra do Windows (ADR 0009):** antes de esconder qualquer janela, a lista das estacionadas é gravada (`parked-windows.json`, ao lado do estado da barra). Elas voltam: na saída normal, num pânico, no reinício por queda do WebView2, no fim da sessão do Windows, pelo vigia (`dockmax-guard.exe`) se o DockMax morrer (ele sobe uma vez por execução, também com a barra visível), e na próxima abertura. Uma janela que não pôde ser anotada não é escondida.
- **O espaço de cada janela sobrevive a um reinício (decisão do autor, 2026-10-27):** como o `session.json` do PrimoDock, o DockMax guarda o espaço de cada janela (`window-workspaces.json`), com o identificador da janela e o do processo dela, para não confundir uma janela nova que o Windows numerou igual. Toda saída mostra todas as janelas (é a regra de segurança), e na abertura seguinte cada janela recupera o seu espaço, e as que não são do espaço mostrado no monitor dela são estacionadas de novo.

## 4b. C3 em detalhe: acabamento (feito)

- **"Mover para <espaço>"** no menu de um programa (fixado ou só aberto), com dois ou mais espaços e janelas abertas: uma entrada para cada espaço menos o em uso. As janelas do programa no espaço em uso passam ao escolhido (`move_app_windows`, `move_open_windows`); em seguida cada monitor é acertado com o espaço que mostra: as que estavam na tela são estacionadas, e as que esperavam por aquele espaço num monitor que o mostra voltam. O menu passou a ler os espaços (`list_workspaces` liberado para a janela dos painéis).
- **"Trocar de espaço em todos os monitores"** (Configurações › Geral, `switchEveryMonitor`, desligada): trocar num dock (bolinha, roda, atalho, menu, leitor de tela) troca todos os monitores para o mesmo espaço; as janelas de cada um seguem a regra do C2, e a que vem para a frente é a do monitor que trocou. Desligada, vale a decisão 2 (por monitor).
- **"Indicador na cor do espaço"** (Configurações › Geral, `indicatorFollowsWorkspace`, desligada): a marca dos apps abertos usa a cor do espaço mostrado naquele dock, por cima da cor do indicador escolhida em Aparência.
- **Widgets em vários espaços:** em "Itens do dock", com um espaço escolhido, o grupo **"De outros espaços"** lista os widgets que outros espaços mostram e este não; um clique mostra o widget aqui também (`share_widget`), no fim. Continua sendo uma instância só (a nota é a mesma em todos). Remover tira só deste espaço; sai de vez quando nenhum espaço o tem mais (C1).
- **Bolinhas para o leitor de tela:** cada bolinha é um botão na UI Automation do dock, antes dos itens, chamado "Espaço <nome>" (e "(mostrado)" no do monitor); acioná-lo troca o espaço só naquele dock (antes, acionar um item pelo leitor de tela agia em todos os docks que tinham a mesma chave).
- **Troca animada:** os itens do espaço novo entram deslizando 36 px a partir do lado em que ele está (o seguinte pela direita, o anterior pela esquerda); os que estão nos dois espaços deslizam do lugar antigo. Sem animação quando o Windows pede menos movimento.

## 5. Fora desta fase

Pilhas de widgets e espaçadores (do PrimoDock), a prévia do dock sobre o papel de parede na tela de Configurações, e as áreas de trabalho virtuais do Windows (o DockMax não as usa nem as substitui; os dois convivem).

## 6. Perguntas em aberto para o autor

O autor pediu para seguir sem responder (2026-10-27); valem as propostas até ele decidir (as duas já no C2):

1. Uma janela arrastada para o outro monitor passa para o espaço ativo de lá (proposta) ou continua no espaço de origem? (C2)
2. Clicar num programa fixado cujas janelas estão todas **em outro espaço** abre uma nova janela no espaço atual (proposta) ou troca para o espaço onde ele está? (C2)
3. ~~A paleta de cores~~: oito fixas, a proposta (C1).

## 7. Verificação

- **Verificado (C1, 2026-10-27):**
  - testes do repositório (criar, renomear, recolorir, mover, excluir, itens em dois espaços, ordem do arrasto, espaço por monitor e em uso) e dos comandos;
  - a tela de Configurações (fichas, renomear, cor, excluir, o espaço passado aos comandos);
  - no app real (release), sem mexer no ponteiro: a migração sobre os dados do autor criou o "Principal" com o que o dock tinha; com um segundo espaço vazio ativo no monitor principal, o dock mostrou as bolinhas e nenhum item; a roda do mouse voltou ao "Principal", e o item voltou. Dados e barra restaurados.
- **Não verificado (C1):** o clique nas bolinhas, `Ctrl+Alt+número` e o menu com o mouse e o teclado de verdade; a seção de Configurações com o mouse; dois monitores em espaços diferentes ao mesmo tempo (o segundo estava em tela cheia).
- **Verificado (C2, 2026-10-27):** testes das regras (dono de uma janela nova, de uma que muda de monitor, de uma fechada e de uma estacionada; o que uma troca esconde e o que traz de volta; o que cada dock conta) e do arquivo das estacionadas (gravado, apagado com a última, o vigia só mostra o que o processo dele escondeu).
- **Verificado no app real (C2, 2026-10-27),** com o DockMax de teste, uma janela de teste e as janelas do autor, a roda do mouse mandada ao dock e sem mexer no ponteiro:
  - a troca para o outro espaço escondeu as 4 janelas do monitor principal (a de teste e 3 do autor), e a volta as trouxe;
  - com janelas escondidas, o DockMax derrubado: o vigia mostrou todas;
  - reaberto com o outro espaço na tela, a sessão escondeu as mesmas 4 de novo, e a volta as trouxe;
  - no fim, as janelas visíveis eram as mesmas do começo, sem nenhuma mostrada à mão.
- **Não verificado (C2):** com o mouse e o teclado de verdade; dois monitores em espaços diferentes; uma janela arrastada para o outro monitor; excluir um espaço com janelas escondidas; o fim da sessão do Windows.
- **Verificado (C3, 2026-10-27):** testes das opções no arquivo e no comando de configurações, do widget mostrado em dois espaços (uma instância; tirar de um mantém no outro; espaço ou widget desconhecido recusado), do menu ("Mover para" só com dois espaços e janelas abertas, nunca para o espaço em uso) e de "De outros espaços".
- **Não verificado (C3):** no app real, mover janelas pelo menu, as duas opções, as bolinhas pelo Narrador e a animação. Ver o PR.
- **Testes (C1 e C2):** a migração sobre um banco real (os itens e a ordem iguais no "Principal"), fixar e desafixar por espaço, excluir espaço, o estado de cada monitor, a regra de dono das janelas e a lista de estacionadas (a pura, com janelas de mentira).
- **App real:**
  - criar três espaços e trocar pelas quatro formas;
  - dois monitores em espaços diferentes;
  - janelas somem e voltam, inclusive minimizadas;
  - matar o DockMax com janelas estacionadas: o vigia as devolve;
  - medir a memória.
