# Spec 0006 — Fase B: apps no dock

- **Status:** B1 e B3 implementados e verificados no app real; falta o autor abrir um programa e escolher uma entrada de menu
- **Data:** 2026-10-23
- **Origem:** [spec 0004](0004-paridade-primodock.md) §3.4, §3.7c e §6 (fase B) e [ADR 0009](../adr/0009-dock-substitui-a-barra-de-tarefas.md)

## 1. O problema

Com o dock no lugar da barra do Windows (fase A e spike da barra), ele só tem Configurações, Sobre, a bandeja e os widgets. Não dá para abrir um programa a partir dele, que é o uso principal de uma barra de tarefas.

## 2. Recortes

| Recorte | Entrega | Toca em |
|---|---|---|
| **B1** | Catálogo dos programas instalados, fixar, desafixar, reordenar, abrir ao clicar e o ícone real de cada um | banco (migração 2), `windows/apps.rs`, `commands/apps.rs`, `Dock.tsx`, Configurações |
| **B2** | Formato dos ícones (Placas arredondadas, Mesmo tamanho, Como vieram), animação de abertura (Pulo, Pulso, Tremor), arrastar para reordenar e soltar um programa no dock para fixá-lo | `settings`, `Dock.tsx`, CSS |
| **B3** | Menu do app no dock (botão direito): a Jump List do Windows, Abrir, "Remover do dock" e "Mostrar no Explorador" (§6 e §7) | `windows/jumplist.rs`, `commands/apps.rs`, painel |
| **B4** | Pacotes de ícones, ícone por item e imagem personalizada | a definir |

B1 não depende de apps em execução (fase D): o dock ainda não sabe quais janelas estão abertas, então o indicador de app aberto fica para a fase D.

## 3. B1 em detalhe

### 3.1 A fonte dos programas: a pasta Aplicativos do shell

O catálogo é a pasta `shell:AppsFolder`, a mesma lista de "Todos os aplicativos" do menu Iniciar. Ela traz programas clássicos e aplicativos da Loja do mesmo jeito, cada um com nome, ícone e um **nome de análise** (*parsing name*) que o abre: o AUMID de um app da Loja (`Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`), ou a pasta conhecida mais o caminho, ou o próprio caminho, de um programa clássico.

Spike (2026-10-23, nesta máquina): 197 entradas listadas em cerca de 1 s a frio (0,3 s depois); ícone de 96 px em 20 a 70 ms cada, com transparência correta; nenhuma entrada sem nome de análise. Programas clássicos e da Loja saem pela mesma API (`IShellItem`, `IShellItemImageFactory`), sem ler arquivos `.lnk` nem o registro.

- **Abrir:** `ShellExecuteW` em `shell:AppsFolder\<nome de análise>`, como o menu Iniciar faz.
- **Ícone:** `IShellItemImageFactory::GetImage` (somente ícone, 96 px), em PNG com alfa direto.
- **Validação do id** no Rust antes de qualquer uso: texto de uma linha, sem caractere de controle, sem espaço nas pontas, até 512 caracteres. O prefixo `shell:AppsFolder\` confina o nome ao conjunto de aplicativos: um caminho qualquer não resolve ali.

### 3.2 Dados

Migração 2 do banco (`0002_apps.sql`, o mesmo `dockmax.db` dos widgets), tabela `STRICT`:

| Coluna | Papel |
|---|---|
| `id` | UUID v7 do item, gerado no Rust; nunca reaproveitado |
| `app_id` | nome de análise; único (comparado sem diferenciar maiúsculas) |
| `name` | nome mostrado, lido do próprio shell ao fixar (o dock continua mostrando o programa se ele for desinstalado) |
| `position` | 0, contíguo; a interface recebe a lista já na ordem |
| `created_at` | milissegundos |

Limite de 40 apps. O modo de recuperação e as cópias de segurança do banco (ADR 0008) valem para a tabela nova sem mudança. Com o banco indisponível, a lista de apps é vazia e as Configurações mostram o aviso que já existe.

Os ícones **não** vão para o banco: são lidos do Windows quando o dock abre e guardados só em memória (um `data:` URL de PNG por app, em torno de 10 KB). Assim um programa que muda de ícone na atualização aparece certo na próxima abertura.

### 3.3 Comandos

Todos validam a entrada no Rust (arquitetura §2 e §8). O erro é só um código, sem caminho nem nome.

| Comando | O que faz |
|---|---|
| `list_apps` | a lista fixada e o estado do armazenamento (`AppsSnapshot`) |
| `list_catalog` | o catálogo, em ordem alfabética (assíncrono: roda fora da thread principal) |
| `app_icon(appId)` | o ícone como `data:` URL, ou `null` |
| `pin_app(appId)` | fixa no fim; o nome vem do shell, não da página. Códigos: `invalidApp`, `alreadyPinned`, `limitReached`, `unavailable`, `writeFailed` |
| `unpin_app(id)` | remove; `notFound` se o id não existe |
| `move_app(id, direction)` | uma posição para cima ou para baixo, como os widgets |
| `launch_app(id)` | abre o app fixado; `notFound` ou `launchFailed` |

Depois de cada mudança aceita, o evento `apps-changed` leva o `AppsSnapshot` a todas as janelas, como o `widgets-changed`.

### 3.4 No dock

A ordem passa a ser **apps, widgets, itens do sistema**, com um separador entre grupos que existem (`groups` do `layout.ts` ignora grupo vazio). Cada app é um botão do dock com o ícone real sobre a placa neutra do tema; o rótulo é o nome. Clicar abre o programa. O ícone ainda não chegou, ou não existe: a inicial do nome sobre a placa.

Sem apps o dock fica como é hoje.

### 3.5 Nas Configurações

Uma linha nova, **Apps do dock**, abre uma subvisão (padrão de "Itens do dock"):

- **Fixados:** lista na ordem do dock, cada linha com o ícone, o nome, subir, descer e remover.
- **Adicionar:** busca por nome sobre o catálogo (sem diferenciar maiúsculas nem acentos) e a lista; clicar fixa. Os já fixados aparecem marcados e não se fixam duas vezes.

### 3.6 Fora de B1

Reordenar arrastando, soltar um programa no dock, menus de contexto, indicador de app aberto, ativar ou minimizar janelas, e pacotes de ícones. Programas que não estão na pasta Aplicativos (um `.exe` solto, um atalho de área de trabalho) só entram em B2, com o soltar.

## 4. Testes

- **Rust:** domínio (limites, validação do id), repositório (fixar, duplicado sem diferenciar maiúsculas, remover renumera, mover nas pontas, limite), migração 2 sobre um banco da versão 1 (os widgets sobrevivem), comandos sem Tauri, e o guarda de capacidades.
- **TS:** o dock com apps (ordem dos grupos, ícone, inicial de reserva, clique abre), o painel (fixar, desafixar, mover, busca, erro) e o contrato de erros.
- **App real:** fixar alguns programas, ver os ícones no dock e medir a memória (meta do projeto). Abrir um programa a partir do dock só com o aviso e a autorização do autor.

## 5. Verificação

Feita em 2026-10-23, no app real (build de depuração), com o banco e as configurações do autor guardados antes e devolvidos depois. Sem mouse, sem menu e sem abrir nenhum programa: os fixados vêm dos comandos do próprio painel, pela porta de depuração do WebView2.

- **Catálogo:** 197 programas em 0,6 s a frio, pela página do painel.
- **Dock:** Calculadora, Bloco de notas e Claude fixados aparecem antes dos widgets, com ícone de 96 × 96 e três separadores (apps, widgets, sistema e relógio). Mover o terceiro para cima reordena o dock.
- **Painel:** a linha Apps do dock abre a subvisão com os três fixados na ordem do dock e 80 programas do catálogo (o limite de linhas); só 9 ícones foram lidos, os das linhas à vista (a leitura é preguiçosa).
- **Banco:** a migração 2 rodou sobre o banco real (versão 1) sem perder widgets; o teste de unidade cobre a cópia de segurança `bak-v1`.
- **Memória** (privada, dock e processos filhos): 173–177 MB parado, 208–220 MB com o painel aberto e três apps fixados. O painel sozinho custa uns 20 MB do processo dele, e o processo do dock cresce uns 6 MB ao carregar o código do shell que extrai ícones. A meta de ~220 MB do roadmap está no limite: qualquer item novo de interface precisa de medida.
- **Restauração:** a barra do Windows voltou e o arquivo de estado foi removido ao encerrar.

Pendente do autor: abrir um programa a partir do dock (clique no ícone), que o teste automático não faz.

## 6. Spike do menu dos apps: as Jump Lists (B3)

Feito em 2026-10-23, só leitura: nada foi aberto nem alterado (`src-tauri/examples/jumplist_probe.rs` e `jumplist_tasks.rs`). Responde ao requisito do autor (spec 0004 §3.7c): o menu de um app no dock deve ser o que a barra do Windows mostra.

### 6.1 O que a barra mostra e de onde vem

O menu de um ícone da barra é a **Jump List** do app: tarefas (por exemplo "Nova janela"), categorias que o próprio app monta e as listas Recentes e Frequentes. O Windows não tem API para desenhar esse menu, só para ler e escrever as listas. O que se pode ler, e como:

| Parte | Como | Resultado na máquina do autor (197 programas) |
|---|---|---|
| **AppID** do programa | propriedade `System.AppUserModel.ID` do item da pasta Aplicativos (`SHGetPropertyStoreFromParsingName`) | todos os 197 têm; programas clássicos sem AUMID próprio devolvem o caminho do executável |
| **Recentes e Frequentes** | API documentada `IApplicationDocumentLists` (`SetAppID`, `GetList`) | 31 programas com itens, entre eles Explorador de Arquivos, Visual Studio Code, Word, Excel, Bloco de notas, Fotos, Chrome e Edge; devolve `IShellItem` |
| **Tarefas e categorias do app** | arquivo `%APPDATA%\Microsoft\Windows\Recent\CustomDestinations\<hash>.customDestinations-ms` | 23 programas com lista, entre eles Chrome, Edge, Visual Studio Code, Calculadora (os cinco modos), Outlook ("Nova Mensagem", "Novo Evento", "Abrir Calendário"), ChatGPT e Claude |

- **O nome do arquivo** é o hash do AppID: CRC-64 de polinômio `0x259C84CBA6426349`, refletido, valor inicial de todos os bits ligados, sem xor final, sobre o AppID em **maiúsculas** como UTF-16 little endian, escrito com 16 dígitos hexadecimais minúsculos. Achado por tentativa (48 acertos em 197 programas, nenhum falso) e confirmado: o arquivo de cada programa com lista aparece pelo hash.
- **O arquivo** é uma sequência de atalhos `.lnk` (cada um começa pelo cabeçalho `4C 00 00 00 01 14 02 00 ...`) entre cabeçalhos de categoria e o marcador `AB FB BF BA`. Basta achar o cabeçalho de cada atalho e entregar o trecho até o próximo a um `IShellLinkW` carregado de um arquivo temporário (`IPersistFile::Load`): ele devolve o **título** (`System.Title`), o destino e os argumentos. Para apps da Loja o destino é o AUMID e os argumentos de ativação ficam nas propriedades do atalho.
- **Para executar** uma entrada: gravar o trecho num `.lnk` temporário e abri-lo com `ShellExecuteW`, que é o que o shell faz; assim nenhuma regra de ativação (Loja, argumentos, diretório de trabalho) é reimplementada.

### 6.2 O que ficou claro

- É viável e dá o mesmo conteúdo da barra para os programas que têm lista, sem reimplementar nada do shell.
- Os títulos podem ser recursos indiretos (`@{pacote?ms-resource:///...}`, como nos modos da Calculadora): resolvem com `SHLoadIndirectString`. Entradas de documento (Word, Excel) vêm sem título: usa-se o nome do arquivo.
- O formato do arquivo não é documentado, mas é o mesmo desde o Windows 7. A leitura tem de ser tolerante: uma entrada que não carrega é pulada e um arquivo que não bate com a assinatura vira "sem lista".
- Pouco mais de um décimo dos programas tem lista própria; os outros ficam só com as ações do dock (abrir, remover, mostrar no Explorador, fechar).
- **Privacidade:** as listas guardam documentos, endereços e conversas recentes. Só são lidas quando o menu abre, nada vai para o log, para o banco ou para a página além do que o menu mostra naquele instante.
- **Não fazer:** `ICustomDestinationList::BeginList` num AppID de outro programa reescreveria a lista dele.

### 6.3 Desenho proposto para o B3

1. Rust: `windows/jumplist.rs` com `destinations(app_id)` (Recentes e Frequentes pela API), `tasks(app_id)` (arquivo e atalhos), `run(entrada)` (atalho temporário e `ShellExecuteW`), todos tolerantes e testados com arquivos de exemplo gravados no formato real.
2. Comando `app_menu(id)` do app fixado, e `run_app_entry`, que só aceita uma entrada que o último `app_menu` desse app devolveu (o menu guarda o que mostrou).
3. Interface: o botão direito num app abre o menu no painel (novo `PanelId`, ancorado no ícone), com as seções Tarefas, Recentes e Frequentes e, no fim, Abrir, Remover do Dock e Mostrar no Explorador de Arquivos.
4. O botão direito no dock ainda não existe: entra junto, com a hospedagem do menu no painel.

## 7. B3 implementado e verificado

O desenho de §6.3 foi implementado como proposto, com estas escolhas:

- **Quem mostra o quê:** um programa com categorias próprias (VS Code, Word, ChatGPT) mostra só elas, mais as tarefas, como a barra; os outros mostram os Recentes, ou os Frequentes quando não há recentes, e as tarefas. Até 10 entradas por lista e 15 tarefas.
- **Títulos:** o `System.Title` do atalho; resolvido por `SHLoadIndirectString` quando é um recurso (os cinco modos da Calculadora saem como "Padrão", "Científica" e assim por diante); sem título, o nome do arquivo de destino.
- **Executar:** `run_app_entry` recebe só a chave do último menu mostrado para aquele app fixado; a página nunca guarda um caminho nem um atalho. Um atalho vira um `.lnk` temporário aberto por `ShellExecuteW`; os temporários com mais de 10 minutos são varridos a cada execução.
- **Mostrar no Explorador:** só para app cujo id é o caminho de um arquivo (programa clássico listado pelo executável); a interface recebe `canReveal` e oculta a ação nos outros.
- **Interface:** o botão direito (ou a tecla de menu) num app fixado abre o painel do menu (`PanelId::App`, ancorado no ícone). Setas, Home e End andam pelos itens; o painel fecha sozinho se o app sair do dock.

Verificação em 2026-10-23, no app real, sem mouse e sem abrir programa (o botão direito foi o evento `contextmenu` da própria página, pela porta de depuração):

- Na leitura das listas desta máquina, 38 dos 197 programas têm menu; o formato de cada um confere com o do spike.
- Calculadora: Tarefas × 5 (os modos, com os títulos resolvidos) e as ações. Chrome: Recentes × 10, Tarefas × 2. VS Code: Pastas Recentes × 10, Tarefas × 2. PowerShell: Tarefas × 3. Bloco de notas: Recentes × 10.
- O evento teve o padrão cancelado nos cinco apps, o painel abriu com o nome do app como título e o ícone ficou `aria-expanded`; a barra do Windows voltou e o arquivo de estado foi removido ao encerrar.

Pendente do autor: escolher uma entrada de um menu (por exemplo "Nova janela" do Chrome), que o teste automático não faz.
