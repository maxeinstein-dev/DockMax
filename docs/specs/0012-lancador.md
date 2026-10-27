# Spec 0012 — Lançador: buscar e abrir pelo teclado

**Status:** rascunho para aprovação. Nenhum código foi escrito.
**Origem:** pedido do autor (2026-10-26) de um lançador no estilo do [Ulauncher](https://ulauncher.io/): uma caixa de busca aberta por um atalho de teclado global, que acha e abre apps e, no futuro, arquivos e extensões. O PrimoDock tem algo parecido ("O botão Iniciar abre: Apps do PrimoDock", [interface](../reference/primodock/interface.md)), mas só pelo mouse e sem busca por teclado.

## 1. Decisões do autor (2026-10-26)

| Pergunta | Decisão |
|---|---|
| Atalho | **Alt+Espaço** por padrão; personalizar fica para depois (recorte L3) |
| Janela | **Nativa** (Direct2D, como o dock), não WebView2 |
| Arquivos | **Só apps** no primeiro recorte; arquivos depois |
| Extras do primeiro recorte | **Todos:** apps recentes com a busca vazia, calculadora, busca na web por palavra-chave, Configurações do DockMax e do Windows |

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **L1** | A janela nativa, o atalho Alt+Espaço (e o item "Buscar apps…" nos menus do dock e da bandeja), a busca nos apps da pasta Aplicativos, a ordenação com o histórico de uso, os recentes com a busca vazia, o teclado, a UI Automation e a chave **Lançador** (ligar e desligar) em Configurações › Geral |
| **L2** | Os outros provedores: **calculadora**, **busca na web por palavra-chave** (com as palavras-chave em Configurações) e **Configurações do DockMax e do Windows** |
| **L3** (depois) | Personalizar o atalho em Configurações |

L1 e L2 formam o primeiro recorte do produto. Cada um é um PR; o L2 só começa depois da medição de memória do L1.

**Fica para depois, fora desta spec:**

- **Arquivos:** navegar por caminho como o Ulauncher (`~\`, `C:\`, Tab entra na pasta) e, depois, o índice do Windows Search.
- **Extensões** (seção 9): só a forma interna dos provedores nasce no L1, preparada para elas.
- A **tecla Win sozinha** ou a tecla Copilot como atalho (exigem um hook de teclado de baixo nível).
- Um menu de ações por resultado: executar como administrador (Ctrl+Shift+Enter, como no Executar do Windows), mostrar no Explorador e fixar no dock.
- Buscar e alternar entre as **janelas abertas**, a área de transferência, a conversão de unidades e o histórico de contas.
- Um item no dock que abre o lançador (o "botão Iniciar" do PrimoDock).

## 3. O atalho

**Alt+Espaço**, registrado com `RegisterHotKey` pela thread do dock (que já tem um laço de mensagens); a janela recebe `WM_HOTKEY`. Apertar de novo com o lançador aberto o fecha.

O Windows dá ao processo que recebe um atalho registrado o direito de trazer uma janela para a frente, então o lançador ganha o foco sem os truques de `AttachThreadInput` que o dock usa para ativar outros programas. Esse ponto precisa de verificação no L1.

**Conflitos conhecidos:**

| Atalho | Quem usa | Situação |
|---|---|---|
| **Alt+Espaço** | O menu de sistema da janela em foco (Restaurar, Mover, Fechar) pelo teclado; o PowerToys Run por padrão | Com o lançador ligado, esse menu deixa de abrir pelo teclado (continua no clique direito da barra de título). O PowerToys não está instalado na máquina do autor. Vai para as [limitações](../limitacoes.md) |
| Ctrl+Espaço | O padrão do Ulauncher; o autocompletar do VS Code e do JetBrains; o IME de chinês e japonês | Evitado |
| Win+Espaço | A troca do idioma de entrada | Reservado pelo Windows |
| Win+S, Win+Q | A busca do Windows | Reservados pelo Explorer, que continua rodando com a barra oculta |
| Win+Alt+Espaço | A Paleta de Comandos do PowerToys | Uma alternativa para o L3 |
| Ctrl+Alt+1…9 | Os espaços de trabalho do PrimoDock (fase C) | Reservado para a fase C |

**Se o registro falhar** (`ERROR_HOTKEY_ALREADY_REGISTERED`, outro programa pegou o atalho primeiro), o DockMax não insiste nem tenta tomá-lo de volta: grava o motivo no log, mostra o aviso de atenção em Configurações › Geral, junto da chave Lançador ("Alt+Espaço está em uso por outro programa"), e o lançador continua abrindo pelo item "Buscar apps…" dos menus do dock e da bandeja. O registro é tentado de novo quando o DockMax abre e quando a chave é religada.

**No L3,** o campo do atalho em Configurações captura uma combinação, desregistra a atual enquanto captura (senão a combinação abre o lançador) e recusa as que não têm modificador, as reservadas pelo Windows (Win+L, Ctrl+Alt+Del, Win+Espaço) e as que falham no registro, dizendo o porquê.

## 4. A janela: nativa, pesando a memória

**Por que nativa.** O dock nativo pesa 64,5 MB, num processo só ([spec 0011](0011-arquitetura-hibrida.md) §7). Um lançador em WebView2 teria três saídas, todas ruins para algo aberto pelo teclado a qualquer hora:

| Opção | Abrir | Memória |
|---|---|---|
| WebView2 sob demanda | 350–390 ms a frio: as teclas digitadas antes da página pintar precisariam ser guardadas | ~85 MB a mais enquanto aberto |
| WebView2 sempre morno | Na hora | Desfaz a spec 0011: o WebView2 fica carregado em repouso |
| **Nativa (escolhida)** | Poucos ms | Alguns MB enquanto aberta (meta na seção 8) |

**Como ela é** (`windows/launcher/`):

- Uma janela pop-up comum, **não** em camadas, sem entrada na barra de tarefas (`WS_EX_TOOLWINDOW`), sempre no topo, com os cantos arredondados do Windows 11 pelo DWM (`DWMWA_WINDOW_CORNER_PREFERENCE`). Não pode ser em camadas porque abriga um controle filho: uma janela de `UpdateLayeredWindow` não mostra controles filhos. **O fundo é uma placa sólida, sem acrílico** (mudança no L1): a caixa de texto do Windows pinta com GDI, sem transparência, e sobre o acrílico o texto sairia errado.
- **A caixa de texto é um `EDIT` do Windows**, subclassificado para Cima, Baixo, Enter, Esc e Alt+número. Com isso vêm de graça o IME, colar, desfazer, a seleção, o cursor e o papel de campo de edição na UI Automation. Desenhar a própria caixa de texto em Direct2D exigiria refazer tudo isso, inclusive o TSF do IME.
- **A lista de resultados é desenhada com Direct2D e DirectWrite**, numa janela filha própria, com o Direct2D **por software** (mudança no L1: o de hardware carregava um dispositivo Direct3D, uns 25 MB, para oito linhas).
- **Tamanho e lugar:** 640 pixels lógicos de largura, a caixa com 56 de altura, até **8 resultados** de 48 cada; centrada na horizontal e a um quarto da altura do monitor **onde está o ponteiro** (como o Ulauncher), no DPI desse monitor.
- **Visual:** segue o claro ou escuro do Windows, com o mesmo destaque do dock. A referência é a caixa do Ulauncher com o tema WhiteSur: uma placa arredondada translúcida, o ícone do app à esquerda, o nome e, embaixo, um subtítulo menor e mais apagado ("App", "Configurações do Windows", "Calculadora").
- **Fecha** com Esc, ao perder o foco (`WM_ACTIVATE` inativo), depois de abrir um resultado e com o atalho de novo. Fechar esconde a janela e limpa o texto; não guarda a última busca.
- **Liberar:** os recursos do Direct2D e o cache de ícones saem **45 s** depois de fechada, como o painel ([spec 0011](0011-arquitetura-hibrida.md) §5). As janelas ficam (o Alt+Espaço é registrado nelas e elas custam quase nada). O catálogo de apps (nomes e ids, poucas dezenas de KB) fica em memória.
- **Com o dock oculto automaticamente** ou um monitor em tela cheia, o lançador abre do mesmo jeito: é uma janela do usuário, não parte do dock.

**O catálogo de apps.** `apps::catalog()` demora (é uma enumeração da pasta Aplicativos pelo shell). No primeiro uso, a caixa abre na hora e a lista mostra "Lendo os apps…" até o catálogo chegar, lido numa thread de trabalho. Nas aberturas seguintes, o catálogo em memória responde na hora e é relido em segundo plano, trocado quando chegar (um app instalado aparece na abertura seguinte à leitura). Ler o catálogo já ao abrir o DockMax é uma alternativa a medir no L1: a enumeração carrega DLLs do shell, e a memória que elas somam em repouso decide.

**Ícones** só dos resultados visíveis, num cache de até 64, descartado ao liberar a janela. O do pacote de ícones escolhido é um arquivo e é lido no processo. O do Windows é lido **num processo auxiliar** (decisão do autor, 2026-10-27): o próprio `dockmax.exe` com `--launcher-icons`, que vive enquanto o lançador está aberto e sai quando ele fecha. Ler o ícone de um app da Loja pelo shell deixava uns 20 MB presos no processo depois de algumas buscas.

## 5. Busca e ordenação

**Normalização** (consulta e nomes): minúsculas, sem acentos (`configuracoes` acha "Configurações", `area` acha "Área de Trabalho Remota"), espaços repetidos viram um.

**Faixas de casamento**, da melhor para a pior; um nome que não cai em nenhuma fica de fora:

1. o nome igual à consulta;
2. o nome começa com a consulta;
3. uma palavra do nome começa com a consulta (`code` acha "Visual Studio Code");
4. as iniciais das palavras (`vsc` acha "Visual Studio Code", `gdt` acha "Gerenciador de Tarefas");
5. a consulta aparece no meio do nome, ou o nome do arquivo do programa começa com ela (`msedge` acha "Microsoft Edge"; vale para os apps clássicos, cujo id é um caminho);
6. as letras da consulta aparecem em ordem no nome (`ntpd` acha "Notepad++"), com no máximo 3 letras do nome entre duas da consulta.

**Ordem:** pela faixa; dentro da faixa, pelo **uso** (a frecência, abaixo), depois o nome mais curto e por fim a ordem alfabética. O uso nunca passa por cima de uma faixa melhor: digitar o nome exato de um app nunca perde para um app muito usado. A regra é pura (`domain/launcher_search.rs`), sem Windows, testada como as do dock.

**O uso** (`launcher_usage` no banco, com migração: o id do resultado, quantas vezes foi aberto pelo lançador e quando foi a última). A frecência é a contagem, cada abertura pesando metade a cada 14 dias. Só o que é aberto **pelo lançador** conta; o dock não escreve nessa tabela. Em Configurações, "Limpar o histórico do lançador" apaga a tabela.

**Busca vazia:** os **8 mais usados** pela frecência, com o título "Recentes". Sem histórico, só a dica "Digite para buscar apps".

**Os provedores do L2**, misturados na mesma lista:

- **Calculadora:** quando a consulta é uma conta (números, `+ - * / ^ %`, parênteses, a vírgula ou o ponto decimal, pelo menos um operador), o resultado vem **primeiro**, no formato regional ("= 3,5"). Enter copia o número para a área de transferência. O analisador é próprio e só faz contas: nada é executado. Divisão por zero e conta incompleta não geram resultado.
- **Busca na web:** uma consulta que começa com uma palavra-chave e um espaço (`g dockmax`) dá um resultado só: "Buscar 'dockmax' no Google". As palavras-chave vêm prontas (`g` Google, `yt` YouTube, `w` Wikipédia, `gh` GitHub) e são editáveis em Configurações: palavra, nome e um modelo com `%s`. O modelo **precisa** começar com `https://`, e o termo é codificado para URL. Quando nada casa, o último resultado é "Buscar 'consulta' na web" no buscador padrão (o primeiro da lista).
- **Configurações:** uma lista fixa, no código, com as páginas `ms-settings:` mais usadas (Wi-Fi, Bluetooth, Som, Vídeo, Energia, Atualizações, Aplicativos, Padrões…) e as seções das Configurações do DockMax, cada uma com o nome em português e sinônimos (`wifi`, `internet` e `rede` acham "Wi-Fi"). Elas casam pelas mesmas faixas que os apps e, numa mesma faixa, vêm depois deles.

**Os provedores por dentro:** cada fonte é um provedor que recebe a consulta e devolve resultados com título, subtítulo, ícone, faixa e **uma ação de uma lista fechada**: abrir um app do catálogo (pelo id, validado por `is_valid_app_id`), abrir uma página das Configurações do DockMax, abrir uma URI de uma lista permitida (`ms-settings:` da lista fixa, `https://` das palavras-chave) ou copiar um texto. Nenhum provedor executa outra coisa, o que mantém a regra do roadmap ("Nenhum comando arbitrário pode ser executado diretamente a partir de conteúdo não confiável") e é a porta das extensões (seção 9).

**Abrir** um app usa o mesmo caminho do dock (`apps::launch`, fora da thread principal); se falhar, o lançador fica aberto e o resultado mostra "Não foi possível abrir" no subtítulo.

## 6. Teclado

| Tecla | Ação |
|---|---|
| Digitar | Busca a cada tecla (o catálogo cabe em memória; sem atraso) |
| Cima, Baixo | Muda o resultado selecionado, sem sair da caixa de texto |
| Enter | Abre o selecionado (o primeiro, se nenhum foi escolhido) |
| Alt+1…9 | Abre o n-ésimo resultado, como no Ulauncher; o atalho aparece sempre à direita de cada linha (no L1, sempre, e não só com Alt apertado) |
| Esc | Fecha |
| Page Up, Page Down, Home, End na lista | Ficam com a caixa de texto (o cursor); a lista tem só 8 linhas |

O mouse também funciona: passar o ponteiro seleciona e o clique abre.

## 7. Acessibilidade

- **A caixa de texto** é um `EDIT` com um provedor de UI Automation próprio (no L1, a caixa do Windows aparecia como um painel sem nome): campo de edição "Buscar apps", com o valor digitado.
- **A lista** tem um provedor de **UI Automation** próprio, como o do dock: uma lista "Resultados" com um item por resultado (nome e subtítulo, a posição "3 de 8", o padrão **SelectionItem** marcando o selecionado e o **Invoke**, que abre).
- **Anúncios:** mudar a seleção levanta o evento de item selecionado, que o Narrador lê ("Visual Studio Code, App, 1 de 8"); depois de uma pausa de 300 ms na digitação, um evento de notificação (`UiaRaiseNotificationEvent`) diz "8 resultados" ou "Nenhum resultado".
- **Alto contraste:** com o modo de contraste do Windows ligado, a janela troca o acrílico e as cores do tema pelas cores do sistema (`COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_HIGHLIGHT`) e marca o selecionado com borda, não só com cor.
- **Movimento:** sem animação de abrir e fechar (no L1 não há esmaecer nenhum).
- **Tamanho do texto:** a fonte segue o "Tamanho do texto" da Acessibilidade do Windows, além do DPI; as linhas crescem com ela.
- **Só teclado:** tudo o que o mouse faz no lançador também se faz pelo teclado (seção 6).

## 8. Como verificar

**Testes de unidade (Rust, sem Windows):**

- a normalização (acentos, maiúsculas, espaços) e cada faixa de casamento, com nomes reais do catálogo do autor;
- a ordem: a faixa vence o uso, o uso vence o comprimento, o desempate alfabético; a frecência pesando metade a cada 14 dias;
- os recentes com a busca vazia (no máximo 8, sem histórico vira a dica);
- L2: a calculadora (precedência, parênteses, `^`, `%`, a vírgula decimal, divisão por zero, conta incompleta, uma consulta que não é conta), as palavras-chave (só `https://`, o termo codificado, uma palavra-chave sem termo, o buscador padrão quando nada casa), as Configurações (sinônimos, a ordem depois dos apps na mesma faixa);
- a lista fechada de ações: um teste que falha se aparecer uma ação nova sem passar pela validação;
- a migração de `launcher_usage` e "Limpar o histórico".

**Ponta a ponta:** comandos só do build de teste, como os do dock: abrir o lançador, digitar uma consulta e ler os resultados (título, subtítulo, selecionado), e apertar um resultado. Cobrem a busca, a seleção com Cima e Baixo e o fechar com Esc.

**No app real** (release; **avisar o autor antes**, porque o teste mostra o lançador na tela e usa o teclado; não rodar com um DockMax do autor aberto):

1. Alt+Espaço abre o lançador **com o foco na caixa**, no monitor do ponteiro, sobre um programa comum e sobre um programa elevado (administrador).
2. Digitar `bloco` deixa o Bloco de Notas primeiro; Enter o abre e o lançador fecha. O teste fecha **só o processo que ele abriu** (pelo PID que nasceu), nunca outro Bloco de Notas.
3. `vsc`, `configuracoes` e `ntpd` acham o que a seção 5 diz.
4. Um programa auxiliar registra Alt+Espaço antes do DockMax abrir: o aviso aparece em Configurações e "Buscar apps…" no menu do dock abre o lançador.
5. A UI Automation (um script que lê a árvore): a janela com o campo "Buscar apps" e a lista "Resultados" com os itens, a seleção e o Invoke funcionando.
6. Claro, escuro e alto contraste; escala de 100, 125 e 150 %; dois monitores.
7. L2: `2+2*3` dá 8 e Enter copia; `g dockmax` abre o navegador na busca; `wifi` abre a página do Wi-Fi.

**Medição** (release, na máquina do autor, contra os 64,5 MB do dock nativo, sem painel aberto):

| Situação | Meta |
|---|---|
| Em repouso, lançador nunca aberto | no máximo **+1 MB** (só o atalho registrado) |
| Lançador aberto, com resultados e ícones | no máximo **+10 MB** |
| 60 s depois de fechar | no máximo **+3 MB** (o catálogo e as DLLs do shell que ficam) |
| Do atalho até a janela pintada | **≤ 50 ms** com o catálogo em memória; a caixa em ≤ 50 ms também no primeiro uso, com "Lendo os apps…" |
| Da tecla até a lista nova | **≤ 16 ms** (um quadro) com o catálogo do autor |
| CPU em repouso | continua em 0 % |

Se a primeira linha passar da meta por causa do catálogo lido ao abrir o DockMax, o catálogo passa a ser lido só no primeiro uso.

**Não dá para verificar sem o autor:** o Narrador de verdade, o IME de outro idioma e o atalho com o teclado físico (o teste usa entrada simulada). Esses itens entram nas "Verificações pendentes" do [roadmap](../roadmap.md).

## 8a. Verificado (L1, 2026-10-27)

No build de teste em release (`dockmax-e2e.exe`, dados numa pasta temporária, barra do Windows visível), na máquina do autor, com entrada de teclado simulada:

- Alt+Espaço abre o lançador em **10–23 ms**, em primeiro plano e com o foco na caixa.
- `bloco` → Bloco de notas, `vsc` → Visual Studio Code, `configuracoes` → Configurações, `gdt` → Gerenciador de Tarefas (todos selecionados em primeiro); `zqxjzqxj` → nenhum; `c` mostra 8 resultados e Baixo, Baixo seleciona o terceiro.
- Enter em `mapa de caracteres` abre um processo novo do Mapa de Caracteres e o lançador fecha; o teste fechou só esse processo. Reaberto, o lançador mostra "Recentes" com o Mapa de Caracteres. Esc fecha.
- A UI Automation mostra o campo de edição "Buscar apps" (com o valor digitado e o foco) e a lista "Resultados" com os itens, o tipo "App" e a seleção.
- A barra do Windows ficou igual antes e depois; nenhum processo sobrou.

| Medida (memória privada, processo principal contra o repouso) | Resultado | Meta |
|---|---|---|
| Em repouso, lançador nunca aberto | não medido à parte (sem um build sem o lançador para comparar); o build de teste ficou em 86–88 MB | +1 MB |
| Aberto, depois de buscas que leram os ícones de uns 30 apps | **+7,2 MB**, mais o auxiliar com **9,9 MB** | +10 MB |
| 50 s depois de fechar | **+1,8 MB**; o auxiliar já saiu | +3 MB |

Com o auxiliar, o total aberto passa da meta de 10 MB, mas só enquanto o lançador está na tela.

**Não verificado:** o Narrador de verdade, o IME de outro idioma, o Alt+Espaço com o teclado físico, outro programa segurando o Alt+Espaço, alto contraste, escala de 150 % e dois monitores; a suíte de ponta a ponta (a CI roda).

## 9. Extensões (depois)

Fica aqui só o desenho que o L1 não pode fechar:

- **Ativadas por palavra-chave**, como no Ulauncher: uma extensão declara a palavra (`gh`, `todo`) e só recebe as consultas que começam com ela; a busca de apps nunca espera uma extensão.
- **Um processo à parte** por extensão, falando JSON por linha na entrada e na saída padrão, com tempo limite por consulta (um resultado atrasado é descartado). Uma extensão travada ou que cai não derruba o DockMax.
- **Só as ações da lista fechada** da seção 5: uma extensão devolve resultados, nunca executa nada pelo DockMax. Uma ação nova (abrir um arquivo, rodar um comando) é decisão de uma spec própria, com permissões declaradas num manifesto e aceitas por quem instala.
- **Instalação explícita**, de uma pasta local; sem loja nem download automático.
- **Memória:** o processo de uma extensão nasce na primeira consulta com a palavra-chave dela e sai junto com a janela do lançador, 45 s depois de fechada.

## 10. Pendências

- Atualizar as [limitações](../limitacoes.md) (Alt+Espaço deixa de abrir o menu de sistema da janela pelo teclado) e o [roadmap](../roadmap.md) no PR do L1.
- Medir no L1 o custo de ler o catálogo ao abrir o DockMax (seção 4) e decidir entre ler na abertura ou no primeiro uso.
- Confirmar no L1 que `WM_HOTKEY` dá o foco ao lançador sem `AttachThreadInput`.
