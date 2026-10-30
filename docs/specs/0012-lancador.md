# Spec 0012 — Lançador: buscar e abrir pelo teclado

**Status:** L1 e L2 entregues (PRs #33 e #37); L3 (personalizar o atalho) implementado; L4 (arquivos e pastas) implementado.
**Origem:** pedido do autor (2026-10-26) de um lançador no estilo do [Ulauncher](https://ulauncher.io/): uma caixa de busca aberta por um atalho de teclado global, que acha e abre apps e, no futuro, arquivos e extensões. O PrimoDock tem algo parecido ("O botão Iniciar abre: Apps do PrimoDock", [interface](../reference/primodock/interface.md)), mas só pelo mouse e sem busca por teclado.

## 1. Decisões do autor (2026-10-26)

| Pergunta | Decisão |
|---|---|
| Atalho | **Alt+Espaço** por padrão; personalizar fica para depois (recorte L3) |
| Janela | **Nativa** (Direct2D, como o dock), não WebView2 |
| Arquivos | **Só apps** no primeiro recorte; arquivos no L4 (decisões de 2026-10-30, seção 5a) |
| Extras do primeiro recorte | **Todos:** apps recentes com a busca vazia, calculadora, busca na web por palavra-chave, Configurações do DockMax e do Windows |

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **L1** | A janela nativa, o atalho Alt+Espaço (e o item "Buscar apps…" nos menus do dock e da bandeja), a busca nos apps da pasta Aplicativos, a ordenação com o histórico de uso, os recentes com a busca vazia, o teclado, a UI Automation e a chave **Lançador** (ligar e desligar) em Configurações › Geral |
| **L2** | Os outros provedores: **calculadora**, **busca na web por palavra-chave** (com as palavras-chave em Configurações) e **Configurações do DockMax e do Windows** |
| **L3** | Personalizar o atalho em Configurações |
| **L4** | **Arquivos e pastas:** o Everything (quando roda) e o índice do Windows Search, misturados abaixo dos apps a partir de 3 caracteres, e a palavra-chave `f ` só com eles (seções 5a e 8d) |
| **L5** (depois) | **Caminhos,** como no Ulauncher: digitar `C:\` ou `~\` lista a pasta, e Tab entra na pasta selecionada |
| **L6** (depois) | **Menu de ações por resultado:** Ctrl+Shift+Enter abre como administrador (como no Executar do Windows), mostrar no Explorador, copiar o caminho, fixar no dock |
| **L7** (depois) | Buscar e alternar entre as **janelas abertas** |

L1 e L2 formam o primeiro recorte do produto. Cada um é um PR; o L2 só começa depois da medição de memória do L1. Do L4 em diante, os recortes seguem em sequência, um PR cada.

**Fica para depois, fora desta spec:**

- **Extensões** (seção 9): só a forma interna dos provedores nasce no L1, preparada para elas.
- A **tecla Win sozinha** ou a tecla Copilot como atalho (exigem um hook de teclado de baixo nível).
- A área de transferência, a conversão de unidades e o histórico de contas.
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

**Como o L3 ficou** (`domain/hotkey.rs`, `commands/launcher.rs`, `src/panel/HotkeyField.tsx`):

- **O formato:** o atalho fica em `settings.json` como texto, `launcherHotkey` (padrão `"Alt+Space"`): os modificadores na ordem Ctrl, Alt, Shift, Win, cada um uma vez, e uma tecla. Um arquivo antigo carrega com Alt+Espaço; um atalho inválido escrito à mão é recusado ao salvar e, ao abrir, o lançador usa Alt+Espaço.
- **As teclas:** letras, números, F1–F24 e Espaço, pela tecla virtual do Windows (`KeyboardEvent.keyCode`), que é o que o `RegisterHotKey` recebe: em qualquer layout (ABNT2, AZERTY), o atalho salvo é a tecla apertada. Enter, Tab, setas e o teclado numérico ficam de fora.
- **Recusadas, com o motivo:** sem modificador; Shift sozinho, ou Ctrl sozinho com letra ou número (tirariam a digitação ou atalhos como Ctrl+C de todos os programas; Ctrl+Espaço e Ctrl+F5 valem); Win sozinho com uma tecla (o Windows usa quase todas: Win+L, Win+Espaço, Win+1…); Alt+F4; Ctrl+Alt e um número (a fase C); e a combinação que outro programa já segura.
- **A captura:** o botão mostra o atalho ("Alt+Espaço"); clicado, pede "Pressione a combinação… (Esc cancela)" e o Rust solta o atalho atual (`capture_launcher_hotkey`). Os modificadores sozinhos esperam a tecla; Esc, Tab ou sair do botão cancelam e o atalho volta. Se Configurações fechar no meio, o Rust pega o atalho de volta sozinho depois de 60 s.
- **Salvar** (`set_launcher_hotkey`, assíncrono porque espera a thread principal): com o lançador ligado, o Rust registra a combinação nova **antes** de salvar; se outro programa a segura, devolve o atalho anterior e responde "Essa combinação está em uso por outro programa". Só depois salva nas configurações. "Voltar para Alt+Espaço" aparece quando o atalho é outro.
- **Alt+Espaço na captura:** o Windows transforma Alt+Espaço no menu da janela (Restaurar, Mover, Fechar) antes de a página ver a tecla, e o `preventDefault` do WebView2 não segura. Enquanto a captura roda, a janela de Configurações intercepta esse pedido de menu (`WM_SYSCOMMAND`/`SC_KEYMENU` com espaço), lê os outros modificadores apertados e entrega a combinação ao campo (evento `launcher-hotkey-pressed`). Fora da captura, o menu abre como sempre.
- **Os textos** que citavam Alt+Espaço usam o atalho em uso: "Lançador (Ctrl+Shift+Espaço)", "Ctrl+Shift+Espaço está em uso por outro programa".

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

- **Calculadora** (`domain/launcher_calc.rs`): quando a consulta é uma conta (números, `+ - * / ^ %`, parênteses, a vírgula ou o ponto decimal, pelo menos um operador), o resultado vem **primeiro**, com a marca decimal da região do Windows ("= 3,5", até 10 casas, sem zeros no fim). Enter copia o número (sem o "=") para a área de transferência e o lançador fecha. O analisador é próprio e só faz contas: nada é executado. Divisão por zero e conta incompleta não geram resultado. **No L2:** `%` depois de um número é um centésimo (`200*10%` dá 20; `50%` dá 0,5), `^` vale da direita para a esquerda, `-2^2` dá -4, `×` e `÷` também valem; `-5` e `(5)` sozinhos não são contas; até 32 níveis de parênteses.
- **Busca na web** (`domain/launcher_web.rs`): uma consulta que começa com uma palavra-chave e um espaço (`g dockmax`) dá um resultado só. **No L2,** o resultado se chama "Buscar “dockmax”" com o subtítulo "Google · busca na web" (em vez de "no Google": "no Wikipédia" sairia errado). As palavras-chave vêm prontas (`g` Google, `yt` YouTube, `w` Wikipédia, `gh` GitHub) e são editáveis em Configurações › Geral: palavra (até 16 caracteres, sem espaço, sem repetir), nome (até 40) e um modelo com `%s` (até 500), no máximo 20. O modelo **precisa** começar com `https://` e ter um site antes de qualquer `/`, `?` ou `#`; o termo é codificado para URL (tudo fora de letras, números e `-_.~` vira `%XX`). Uma palavra-chave sem termo (`g ` ou `g`) não busca: a consulta segue para os apps. **Quando nada casa** (nem app, nem página, nem conta), o único resultado é a busca na web com o buscador padrão (o primeiro da lista); sem palavras-chave, "Nenhum resultado". Em Configurações, "Tornar padrão" leva uma palavra-chave para o topo.
- **Configurações** (`domain/launcher_pages.rs`): uma lista fixa, no código, com 34 páginas `ms-settings:` (Wi-Fi, Rede, Bluetooth, Som, Vídeo, Energia, Armazenamento, Atualizações, Aplicativos instalados e padrão, Plano de fundo, Cores…) e as cinco seções das Configurações do DockMax, cada uma com o nome em português e sinônimos (`wifi`, `internet` e `rede sem fio` acham "Wi-Fi"). Elas casam pelas mesmas faixas que os apps (o nome ou qualquer sinônimo, a melhor faixa) e, numa mesma faixa, vêm depois deles, antes do uso. Abrir uma página conta no uso (`setting:<id>`) e ela aparece nos recentes; a conta e a busca na web não contam. Uma seção do DockMax abre a janela de Configurações já nela (um script de início, com o nome da seção de uma lista fechada, numa janela nova; um evento, com a janela aberta).

**Os provedores por dentro:** cada fonte é um provedor que recebe a consulta e devolve resultados com título, subtítulo, ícone, faixa e **uma ação de uma lista fechada** (`launcher::Action`, checada por `launcher::is_allowed` antes de rodar e de novo em `apps::open_uri`): abrir um app do catálogo (pelo id, validado por `is_valid_app_id`), abrir uma página das Configurações do DockMax, abrir uma URI de uma lista permitida (`ms-settings:` da lista fixa, `https://` com site e sem espaços, até 2048 caracteres) ou copiar um texto (até 64 caracteres, sem controle). Os resultados que não são apps mostram um glifo da fonte de ícones do Windows no lugar do ícone (engrenagem, calculadora, globo). Nenhum provedor executa outra coisa, o que mantém a regra do roadmap ("Nenhum comando arbitrário pode ser executado diretamente a partir de conteúdo não confiável") e é a porta das extensões (seção 9).

**Abrir** um app usa o mesmo caminho do dock (`apps::launch`, fora da thread principal); se falhar, o lançador fica aberto e o resultado mostra "Não foi possível abrir" no subtítulo.

## 5a. Arquivos e pastas (L4)

**Decisões do autor (2026-10-30):**

- **Fontes:** o **Everything** (voidtools), quando está rodando, e o **índice do Windows Search**. Com os dois respondendo, os resultados se juntam sem repetir o caminho (comparado sem diferenciar maiúsculas); sem o Everything, só o Windows Search. O Dockyard não instala nem baixa o Everything.
- **Mistura:** a partir de **3 caracteres**, os arquivos e pastas aparecem **abaixo** dos apps, das Configurações e da calculadora, numa faixa própria depois dessas. A palavra-chave **`f `** (f e espaço) mostra **só** arquivos e pastas.
- **Limite:** até **8** arquivos na mistura e até **30** com `f `.
- **Abrir:** Enter abre o arquivo pelo programa padrão e a pasta no Explorador. A linha tem o nome como título e a pasta onde está como subtítulo, com o perfil abreviado em `~`. O ícone vem do tipo do arquivo, pelo processo auxiliar (seção 4), sem carregar o shell no processo principal.
- **Recentes e uso:** abrir um arquivo conta no uso (`file:<caminho>`) e ele aparece nos recentes, como as páginas.

O desenho e o que foi medido estão em "Como o L4 ficou" (seção 8d).

## 6. Teclado

| Tecla | Ação |
|---|---|
| Digitar | Busca a cada tecla (o catálogo cabe em memória; sem atraso); os arquivos, 120 ms depois da última tecla (L4) |
| Cima, Baixo | Muda o resultado selecionado, sem sair da caixa de texto |
| Enter | Abre o selecionado (o primeiro, se nenhum foi escolhido) |
| Alt+1…9 | Abre o n-ésimo resultado, como no Ulauncher; o atalho aparece sempre à direita de cada linha (no L1, sempre, e não só com Alt apertado) |
| Esc | Fecha |
| Page Up, Page Down, Home, End na lista | Ficam com a caixa de texto (o cursor) |
| Roda do mouse | Rola a lista quando há mais de 8 resultados (L4: até 16 na mistura, 30 com `f `); Cima e Baixo também rolam, levando a seleção |

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
- L4: a mistura e a ordem dos arquivos, o `f `, o limite, a junção sem repetir caminho, a abreviação com `~`, o descarte de uma geração antiga, os buffers da IPC do Everything e o escape do termo para cada fonte;
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

## 8b. Verificado (L2, 2026-10-27)

No build de teste em release, na máquina do autor, com entrada de teclado simulada e a barra do Windows visível:

- `2+2*3` → "= 8", `7/2` → "= 3,5", `200*10%` → "= 20", sempre em primeiro e selecionados, com o subtítulo "Calculadora · Enter copia o resultado".
- `g dockmax` → um resultado só, "Buscar “dockmax”" (Google · busca na web); `yt lo-fi` → "Buscar “lo-fi”" (YouTube); `zqxjzqxj` → a busca padrão do Google, sozinha; `g ` (sem termo) → os apps com "g".
- `wifi` → "Wi-Fi" e `papel de parede` → "Plano de fundo", com o subtítulo "Configurações do Windows".
- Enter em `aparencia` ("Aparência do dock", Configurações do DockMax): o lançador fecha e a janela de Configurações abre já na seção Aparência do dock (conferido pela captura da janela). Reaberto, o lançador mostra a página em "Recentes".
- A barra do Windows ficou igual antes e depois; nenhum processo sobrou.

| Medida (memória privada, processo principal) | Resultado | Meta |
|---|---|---|
| Aberto, depois das buscas | **+7,0 MB**, mais o auxiliar de ícones com **8 MB** | +10 MB |
| 50 s depois de fechar | **+2,2 MB** e **+3,4 MB** em duas rodadas, as duas com a janela de Configurações do teste ainda aberta; o auxiliar já saiu | +3 MB |

**Verificado pelo autor** (2026-10-27): o Enter na conta copia o resultado, e o Enter em `g dockmax` e em `wifi` abre o navegador na busca e a página do Wi-Fi.

**Não verificado:** o Narrador e o editor de palavras-chave pelo teclado. Esses itens estão no [roadmap](../roadmap.md).

## 8c. Verificado (L3, 2026-10-27)

No build de teste em release, na máquina do autor, com teclado simulado com os códigos de posição (`MapVirtualKey`), como um teclado de verdade:

1. Em Configurações › Geral, o campo capturou Ctrl+Shift+K e mostrou "Atalho salvo: Ctrl+Shift+K"; o `settings.json` ficou com `"launcherHotkey": "Ctrl+Shift+K"`.
2. Alt+Espaço deixou de abrir o lançador e Ctrl+Shift+K passou a abrir.
3. Com o próprio roteiro segurando Alt+Espaço (como outro programa), "Voltar para Alt+Espaço" foi recusado com "Essa combinação está em uso por outro programa"; o atalho salvo continuou Ctrl+Shift+K e funcionando.
4. Ctrl+C foi recusado com o motivo.
5. Alt+Espaço capturado pelo teclado foi salvo, **sem** abrir o menu da janela, e voltou a abrir o lançador.
6. "Sair" pelo menu do dock de teste: o processo saiu sem queda no log do Windows; a barra ficou igual e nenhum processo sobrou.

Duas falhas apareceram e foram corrigidas antes do PR: o campo lia a **posição** da tecla (`code`) em vez da tecla virtual (o atalho salvo seria outro num layout como o AZERTY), e Alt+Espaço abria o menu da janela em vez de chegar à captura.

**Não verificado:** a captura com o teclado físico, combinações com Win dentro do WebView2 (o Windows segura muitas antes da página), e o leitor de tela no campo.

## 8d. Como o L4 ficou

**Os arquivos** (`domain/launcher_files.rs`, puro e testado; `windows/launcher/files.rs`; o protocolo do auxiliar em `windows/launcher/icons.rs`):

- **Quando:** `launcher_files::parse`. Uma consulta de **3 caracteres ou mais** (normalizada) busca arquivos para misturar, a não ser que seja uma conta ou uma busca na web por palavra-chave, que vêm sozinhas. `f termo` busca só arquivos, e um caractere de termo já basta; `f` e `f ` sem termo são digitação comum (seguem para os apps). **A palavra-chave `f` não colide com as padrão da web** (`g`, `yt`, `w`, `gh`; um teste garante). Se o usuário criar uma palavra-chave da web `f` em Configurações, **a dele vence**: `f termo` vira a busca na web dele e os arquivos só entram pela mistura.
- **A mistura:** os apps e as páginas continuam respondendo na hora, até 8, e a conta continua primeiro. Os arquivos entram **depois de tudo**, até 8; com `f `, sozinhos, até 30. A busca na web padrão ("quando nada casa") só aparece se não houver nem app nem arquivo. A lista mostra 8 linhas e **rola** além disso: Cima e Baixo levam a seleção e a lista junto, a roda do mouse rola, e Alt+1…9 abre a n-ésima linha **visível**.
- **A ordem dos arquivos:** pelas mesmas faixas de casamento dos apps (seção 5), comparando o nome e, para um arquivo, o nome sem a extensão (`relatorio` é o nome exato de "relatorio.txt"); dentro da faixa, pelo uso (`file:<caminho>`); depois, na ordem em que as fontes responderam (o Everything primeiro).
- **O termo vira palavras** (`launcher_files::words`): separado nos espaços e em `" * ? < > | : \ /`, sem caracteres de controle, até 8 palavras de até 64 caracteres. Nenhum desses caracteres existe num nome de arquivo, e sem eles nada do termo vira sintaxe das fontes:
  - **Everything:** cada palavra entre aspas duplas (`"relatorio" "2024"`), que o Everything trata como literal (operadores, curingas e modificadores como `regex:` não valem ali dentro); espaço entre elas é E. Sem as opções de maiúsculas, palavra inteira, caminho ou regex.
  - **Windows Search:** `CONTAINS(System.FileName, '"relatorio*" AND "2024*"')`, com o apóstrofo dobrado (`joe''s`), como pede o SQL. **Por que `CONTAINS` e não `LIKE '%termo%'`:** o `CONTAINS` usa o índice de texto (medido nesta máquina: 64–190 ms, contra 336–510 ms do `LIKE`, que lê todos os nomes por causa do `%` no início), dá a relevância (`System.Search.Rank`) para ordenar e trata acentos como o índice. O que ele perde é uma palavra no meio de outra (`orio`), raro para arquivos.
- **A consulta ao índice:** `SELECT TOP n System.ItemUrl, System.FileAttributes FROM SystemIndex WHERE SCOPE='file:' AND CONTAINS(...) ORDER BY System.Search.Rank DESC, System.DateAccessed DESC`. O caminho sai do `System.ItemUrl` (`file:C:/Users/...`), **não** do `System.ItemPathDisplay`, que traz os nomes traduzidos do Explorador (`C:\Usuários\...`) e não abre nada (achado no teste ao vivo). A pasta vem do bit de diretório dos atributos.
- **O Everything** pela IPC do SDK oficial (`domain/everything_ipc.rs`, com a fonte citada: `ipc/everything_ipc.h` do Everything-SDK.zip): `WM_COPYDATA` com `EVERYTHING_IPC_COPYDATA_QUERY2W` (18) e um `EVERYTHING_IPC_QUERY2` para a janela de classe `EVERYTHING_TASKBAR_NOTIFICATION`, pedindo só o caminho completo (`REQUEST_FULL_PATH_AND_NAME`), ordenado por data de modificação. A resposta vem num `WM_COPYDATA` para uma janela só de mensagens do auxiliar, num `EVERYTHING_IPC_LIST2`, lido com os limites conferidos (um buffer curto ou um deslocamento fora dele não lê nada fora). Sem a janela do Everything, a fonte fica de fora sem erro. Nada da `Everything64.dll`.
- **A junção:** `launcher_files::merge`, Everything primeiro, cada caminho uma vez (sem diferenciar maiúsculas, sem a `\` do fim); um caminho que não é de unidade (`C:\...`) fica de fora.
- **Abrir:** a ação nova `Action::OpenFile(caminho)` da lista fechada, checada por `is_allowed` (um caminho completo numa unidade, sem `.` nem `..` como pasta, sem os caracteres que um caminho não tem, até 1024) e de novo em `apps::open_path`, que chama `ShellExecuteW` "open" numa thread de trabalho, como `apps::launch`: o arquivo abre no programa padrão e a pasta no Explorador. Se falhar (um arquivo apagado depois da busca), a linha mostra "Não foi possível abrir".
- **Uso e recentes:** abrir conta como `file:<caminho>`, com `\` no fim para uma pasta (assim os recentes sabem que é pasta sem perguntar ao disco); `launcher_repo::is_valid_key` aceita a chave. Com a busca vazia, os recentes juntam apps, páginas e arquivos pelo peso.
- **A linha:** o nome do arquivo como título; a pasta onde está como subtítulo, com o perfil (`USERPROFILE`) escrito `~` (`~\Documents\Contratos`), sem diferenciar maiúsculas e só numa fronteira de pasta (`C:\Users\anabela` não vira `~bela`). O ícone é o do tipo (`type:.pdf`, lido uma vez para todos os PDFs) ou o próprio, para pasta, programa, atalho e arquivo sem extensão; enquanto não chega, um glifo de documento ou de pasta.
- **Acessibilidade:** o item da UI Automation tem o tipo "Arquivo" ou "Pasta" (o subtítulo continua na tela); o Narrador lê "relatorio.pdf, Arquivo, 9 de 12". O anúncio "N resultados" sai de novo quando os arquivos chegam. Uma linha fora da área rolada tem a caixa vazia na UI Automation.

**Onde roda:**

| O quê | Processo | Thread |
|---|---|---|
| A busca de apps, a mistura, a ordem final dos arquivos, o debounce e a geração | principal | a da UI (a do lançador) |
| Falar com o auxiliar | principal | `launcher-helper` (o leitor de ícones, que passou a levar também as buscas) |
| O Everything (IPC) | auxiliar (`--launcher-icons`) | a principal do auxiliar, com uma janela só de mensagens |
| O Windows Search (COM e OLE DB) | auxiliar | `launcher-windows-search`, que guarda a sessão entre consultas |
| Os ícones de arquivo e de pasta (shell) | auxiliar | a principal do auxiliar |
| Abrir o arquivo (`ShellExecuteW`) | principal | `launcher-open`, uma por abertura |

- **O debounce e a geração:** cada tecla aumenta a geração e rearma um temporizador de **120 ms**; quando ele dispara, a busca sai com a geração da vez. Uma resposta com geração antiga (a digitação continuou, o lançador fechou) é descartada (`launcher_files::Generation`). Enquanto a busca nova não chega, as linhas de arquivo da busca anterior que ainda casam com o termo ficam, para a lista não piscar.
- **O tempo limite:** cada fonte tem **300 ms**. O Everything responde em poucos ms; o Windows Search, se atrasar, fica de fora daquela resposta, e o resultado atrasado é descartado pelo número da consulta quando chega. O leitor do auxiliar manda primeiro os pedidos de ícone e, das buscas que se acumularam, só a última.
- **Por que no auxiliar:** o mesmo motivo do L1 (o shell prende memória dos ícones no processo) e para o COM e o OLE DB do Windows Search ficarem fora do processo principal. O auxiliar já existia, nasce quando o lançador precisa e sai quando ele fecha, levando essa memória junto.

**Medido** (2026-10-30, na máquina do autor, pelo teste `windows_search_answers`, ignorado no `cargo test` e rodado à mão com `cargo test windows_search_answers -- --ignored --nocapture`): a sessão do Windows Search abre em 2–8 ms; a primeira consulta de `relatorio` (TOP 30) levou 120–190 ms e as seguintes 64–75 ms, com 26 resultados; o `LIKE` levou 336–510 ms. O processo do teste ganhou **+1,1 MB** de memória privada com o COM e o OLE DB carregados, que agora ficam no auxiliar. O Everything não estava instalado: a fonte ficou de fora sem erro.

**Não verificado (só ao vivo):** a memória do processo principal e do auxiliar com o lançador aberto buscando arquivos (a meta da seção 8 continua: +10 MB aberto, +3 MB 60 s depois de fechar); o Everything de verdade (só os buffers foram testados, no formato do SDK); o Everything rodando como administrador (o Windows pode barrar o `WM_COPYDATA` de um processo comum); os ícones de arquivo e de pasta na tela; o Enter abrindo um arquivo e uma pasta; a roda do mouse; o Narrador lendo "Arquivo" e "Pasta"; se o `CONTAINS` acha `relatorio` em "Relatório.docx" (o índice ignora acentos por padrão, mas não havia arquivo assim para conferir).

## 9. Extensões (depois)

Fica aqui só o desenho que o L1 não pode fechar:

- **Ativadas por palavra-chave**, como no Ulauncher: uma extensão declara a palavra (`gh`, `todo`) e só recebe as consultas que começam com ela; a busca de apps nunca espera uma extensão.
- **Um processo à parte** por extensão, falando JSON por linha na entrada e na saída padrão, com tempo limite por consulta (um resultado atrasado é descartado). Uma extensão travada ou que cai não derruba o DockMax.
- **Só as ações da lista fechada** da seção 5: uma extensão devolve resultados, nunca executa nada pelo DockMax. Uma ação nova (rodar um comando; abrir um arquivo entrou no L4, mas só para o que a busca de arquivos acha) é decisão de uma spec própria, com permissões declaradas num manifesto e aceitas por quem instala.
- **Instalação explícita**, de uma pasta local; sem loja nem download automático.
- **Memória:** o processo de uma extensão nasce na primeira consulta com a palavra-chave dela e sai junto com a janela do lançador, 45 s depois de fechada.

## 10. Pendências

- Atualizar as [limitações](../limitacoes.md) (Alt+Espaço deixa de abrir o menu de sistema da janela pelo teclado) e o [roadmap](../roadmap.md) no PR do L1.
- Medir no L1 o custo de ler o catálogo ao abrir o DockMax (seção 4) e decidir entre ler na abertura ou no primeiro uso.
- Confirmar no L1 que `WM_HOTKEY` dá o foco ao lançador sem `AttachThreadInput`.
