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
| **C3. Acabamento** | "Mover para o espaço" no menu do programa; a cor do indicador seguindo o espaço (opção); a opção de troca global em vez de por monitor; widgets em vários espaços pela interface. |

Cada recorte é um PR, com o plano e a verificação no app real, e mede a memória (o dock nativo não deve crescer de forma visível com espaços).

## 3. C1 em detalhe

### 3.1 Dados (migração 3)

```sql
CREATE TABLE workspaces (
  id TEXT PRIMARY KEY NOT NULL,     -- UUID v7
  name TEXT NOT NULL,
  color TEXT NOT NULL,              -- uma cor da paleta, #RRGGBB
  position INTEGER NOT NULL,        -- ordem das bolinhas e do atalho
  created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE workspace_items (
  workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,               -- 'app' (pinned_apps.id) ou 'widget' (widget_instances.id)
  item_id TEXT NOT NULL,
  position INTEGER NOT NULL,        -- ordem no dock, apps e widgets misturados
  PRIMARY KEY (workspace_id, kind, item_id)
) STRICT;
```

- A migração cria o espaço **"Principal"** (cor azul) com todos os programas fixados e widgets de hoje, na ordem de `meta.dock_order`, que deixa de ser usada.
- `pinned_apps` continua com uma linha por programa (`app_id` segue único); o que se repete é o item em cada espaço. Desafixar de um espaço remove o item; a linha em `pinned_apps` some quando nenhum espaço a usa mais. O mesmo vale para os widgets.
- O espaço ativo de cada monitor fica em `meta` (`workspace:<nome do monitor>`, como `\\.\DISPLAY1`); um monitor novo começa no primeiro espaço.
- Excluir um espaço: não pode excluir o último; os itens dele saem (os programas e widgets que nenhum outro espaço usa são removidos, com confirmação que diz quantos); os monitores nele passam para o primeiro espaço.

### 3.2 Troca

- **Bolinhas** no começo do dock (antes dos programas), uma por espaço, na cor dele; a do ativo é maior e cheia, as outras vazadas. Só com dois ou mais espaços. Clique troca; o nome aparece como rótulo ao passar o mouse; a UI Automation as apresenta como botões ("Espaço Trabalho, ativo").
- **Roda do mouse** sobre o dock: próximo ou anterior, sem dar a volta.
- **`Ctrl+Alt+1…9`** (`RegisterHotKey`): vai ao espaço daquela posição **no monitor onde está o ponteiro**. Se o Windows recusar um atalho (outro programa já o tem), o DockMax segue sem ele e registra no log.
- **Menu do dock** (botão direito no espaço vazio): seção "Espaços" com cada espaço, a marca no ativo e o atalho à direita.
- A troca anima os itens: os que saem e os que entram deslizam como no arrasto (spec 0011, `slide`).

### 3.3 Configurações

Uma seção nova, **"Espaços de trabalho"**: a lista dos espaços (bolinha, nome, quantos apps e widgets), com renomear, mudar a cor, subir e descer, excluir, e "+ Novo espaço" (nasce vazio, com a próxima cor da paleta). Uma frase explica: "Cada espaço tem o seu dock e guarda as suas janelas. Troque pelas bolinhas, pela roda do mouse sobre o dock ou com Ctrl+Alt+número."

Os itens de cada espaço se arrumam no próprio dock (fixar, arrastar, desafixar), com o espaço ativo.

## 4. C2 em detalhe: janelas por espaço

- **Dono:** uma janela de programa (as que a barra de tarefas mostraria, o mesmo critério dos apps abertos da spec 0009) pertence ao espaço ativo no monitor dela **quando apareceu**. As que já estavam abertas quando o DockMax abriu pertencem ao espaço ativo do monitor delas.
- **Trocar** de A para B num monitor: as janelas de A naquele monitor que estão visíveis são **estacionadas** (escondidas com `ShowWindow(SW_HIDE)`, o que as tira também do Alt+Tab); as de B estacionadas naquele monitor voltam como estavam (minimizadas continuam minimizadas). A janela que estava na frente em B volta à frente.
- **Uma janela levada a outro monitor** passa a ser do espaço ativo dele (pergunta em aberto 1).
- **Segurança, como a barra do Windows (ADR 0009):** a lista das janelas estacionadas é gravada num arquivo de estado antes de esconder qualquer uma; o vigia (`dockmax-guard.exe`) mostra todas de volta se o DockMax cair; a saída normal, o fim da sessão do Windows e a próxima abertura também mostram todas. Nunca se esconde uma janela sem antes gravá-la.
- **Apps abertos no dock:** só os do espaço ativo no monitor. O indicador de um programa fixado conta só as janelas dele nesse espaço.

## 5. Fora desta fase

Pilhas de widgets e espaçadores (do PrimoDock), a prévia do dock sobre o papel de parede na tela de Configurações, e as áreas de trabalho virtuais do Windows (o DockMax não as usa nem as substitui; os dois convivem).

## 6. Perguntas em aberto para o autor

1. Uma janela arrastada para o outro monitor deve passar para o espaço ativo de lá (proposta) ou continuar no espaço de origem?
2. Clicar num programa fixado cujas janelas estão todas **em outro espaço**: abrir uma nova janela no espaço atual (proposta, como se não estivesse aberto) ou trocar para o espaço onde ele está?
3. A paleta de cores: as 3 do PrimoDock (azul `#0A84FF`, laranja `#FF9F0A`, verde `#30D158`) mais umas 5 (proposta: roxo, rosa, vermelho, amarelo, cinza), ou uma cor livre?

## 7. Verificação

- **Testes:** a migração 3 sobre um banco real de versão 2 (os itens e a ordem iguais no "Principal"), fixar e desafixar por espaço, excluir espaço, o estado de cada monitor, a regra de dono das janelas e a lista de estacionadas (a pura, com janelas de mentira).
- **App real:**
  - criar três espaços e trocar pelas quatro formas;
  - dois monitores em espaços diferentes;
  - janelas somem e voltam, inclusive minimizadas;
  - matar o DockMax com janelas estacionadas: o vigia as devolve;
  - medir a memória.
