# Spec 0012 — Lançador: buscar e abrir pelo teclado

**Status:** L1 e L2 entregues (PRs #33 e #37); L3 (personalizar o atalho) implementado; L4 (arquivos e pastas) implementado; L5 (navegar por caminho) implementado; L6 (menu de ações por resultado) implementado; L7 (janelas abertas) implementado; a área de transferência (seção 8h) implementada.
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
| **L5** | **Caminhos,** como no Ulauncher: digitar `C:\`, `~\`, `%APPDATA%\` ou `\\servidor\compartilhamento\` lista a pasta, Tab completa e entra na pasta selecionada, Backspace sobe uma pasta (seção 8e) |
| **L6** | **Menu de ações por resultado:** Ctrl+Shift+Enter abre como administrador (como no Executar do Windows), mostrar no Explorador, copiar o caminho, fixar no dock |
| **L7** | Buscar e alternar entre as **janelas abertas**, misturadas entre os apps e os arquivos, e a palavra-chave `j ` só com elas (seção 8g) |

L1 e L2 formam o primeiro recorte do produto. Cada um é um PR; o L2 só começa depois da medição de memória do L1. Do L4 em diante, os recortes seguem em sequência, um PR cada.

**Fica para depois, fora desta spec:**

- **Extensões** (seção 9): só a forma interna dos provedores nasce no L1, preparada para elas.
- A **tecla Win sozinha** ou a tecla Copilot como atalho (exigem um hook de teclado de baixo nível).
- A conversão de unidades e o histórico de contas. (A área de transferência saiu desta lista: seção 8h.)
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
| Digitar | Busca a cada tecla (o catálogo cabe em memória; sem atraso), relendo as janelas abertas (L7); os arquivos, 120 ms depois da última tecla (L4) |
| `j ` | Só as janelas abertas (L7); sem nada depois, todas, a usada por último primeiro |
| `cb ` | Só o histórico da área de transferência do Windows (seção 8h); sem nada depois, os últimos copiados. Enter põe o item de volta como o copiado agora, sem colar |
| Cima, Baixo | Muda o resultado selecionado, sem sair da caixa de texto |
| Enter | Abre o selecionado (o primeiro, se nenhum foi escolhido); numa janela (L7), traz para a frente, restaurando se estiver minimizada |
| Shift+Enter | O menu de ações do selecionado (L6); numa janela: Alternar para a janela, Fechar janela, Mostrar no Explorador (L7); numa página de Configurações, numa busca na web ou numa conta, que só têm uma ação, faz essa ação direto, sem menu |
| Tecla Menu, Shift+F10 | O menu de ações do selecionado (L6); sem menu (página, busca na web, conta, nenhum resultado), o menu do próprio campo de texto, como antes |
| Ctrl+Shift+Enter | Abre o selecionado como administrador, como no Executar do Windows (L6): app, programa, script, instalador ou atalho; nos outros (inclusive uma janela), nada |
| Ctrl+C | Sem texto selecionado no campo, copia o caminho do arquivo ou da pasta selecionada (ou o resultado da conta), fecha e diz "Caminho copiado" (L6); com texto selecionado, copia o texto, como sempre. Numa janela, o Ctrl+C da caixa de texto |
| Alt+1…9 | Abre o n-ésimo resultado, como no Ulauncher; o atalho aparece sempre à direita de cada linha (no L1, sempre, e não só com Alt apertado) |
| Tab | Num arquivo ou numa pasta, completa o caminho no campo; numa pasta, acrescenta `\` e entra nela (L5). Nos outros resultados, nada. Shift+Tab, nada |
| Backspace | Com o cursor no fim de um caminho terminado em `\`, volta para a pasta de cima (L5); fora disso, apaga um caractere |
| Esc | Fecha |
| Page Up, Page Down, Home, End na lista | Ficam com a caixa de texto (o cursor) |
| Roda do mouse | Rola a lista quando há mais de 8 resultados (L4: até 16 na mistura, 30 com `f `; L7: até 24 na mistura, com 8 janelas, e 30 com `j `); Cima e Baixo também rolam, levando a seleção |

O mouse também funciona: passar o ponteiro seleciona, o clique abre e o botão direito seleciona a linha e abre o menu de ações no ponteiro (L6).

## 7. Acessibilidade

- **A caixa de texto** é um `EDIT` com um provedor de UI Automation próprio (no L1, a caixa do Windows aparecia como um painel sem nome): campo de edição "Buscar apps", com o valor digitado.
- **A lista** tem um provedor de **UI Automation** próprio, como o do dock: uma lista "Resultados" com um item por resultado (nome e subtítulo, a posição "3 de 8", o padrão **SelectionItem** marcando o selecionado e o **Invoke**, que abre).
- **Anúncios:** mudar a seleção levanta o evento de item selecionado, que o Narrador lê ("Visual Studio Code, App, 1 de 8"); depois de uma pausa de 300 ms na digitação, um evento de notificação (`UiaRaiseNotificationEvent`) diz "8 resultados" ou "Nenhum resultado".
- **Alto contraste:** com o modo de contraste do Windows ligado, a janela troca o acrílico e as cores do tema pelas cores do sistema (`COLOR_WINDOW`, `COLOR_WINDOWTEXT`, `COLOR_HIGHLIGHT`) e marca o selecionado com borda, não só com cor.
- **Movimento:** sem animação de abrir e fechar (no L1 não há esmaecer nenhum).
- **Tamanho do texto:** a fonte segue o "Tamanho do texto" da Acessibilidade do Windows, além do DPI; as linhas crescem com ela.
- **Só teclado:** tudo o que o mouse faz no lançador também se faz pelo teclado (seção 6).
- **O menu de ações (L6)** é o menu próprio do app (`windows/menu`, o do dock), com o mesmo provedor de UI Automation: o Narrador ouve o menu abrir ("Dockyard") e lê cada item com o foco ("Abrir como administrador", "Mostrar no Explorador"); o Invoke escolhe. Aberto pelo teclado, o primeiro item já vem com o foco. "Caminho copiado" é um anúncio, como "Resultado copiado".
- **A área de transferência (seção 8h):** o item da UI Automation tem o tipo "Área de transferência" e a primeira linha como nome; o Narrador lê "Relatório de vendas, Área de transferência, 1 de 12". Pôr um item de volta diz "Copiado"; a falha de leitura é dita como as outras notas da lista.

## 8. Como verificar

**Testes de unidade (Rust, sem Windows):**

- a normalização (acentos, maiúsculas, espaços) e cada faixa de casamento, com nomes reais do catálogo do autor;
- a ordem: a faixa vence o uso, o uso vence o comprimento, o desempate alfabético; a frecência pesando metade a cada 14 dias;
- os recentes com a busca vazia (no máximo 8, sem histórico vira a dica);
- L2: a calculadora (precedência, parênteses, `^`, `%`, a vírgula decimal, divisão por zero, conta incompleta, uma consulta que não é conta), as palavras-chave (só `https://`, o termo codificado, uma palavra-chave sem termo, o buscador padrão quando nada casa), as Configurações (sinônimos, a ordem depois dos apps na mesma faixa);
- a lista fechada de ações: um teste que falha se aparecer uma ação nova sem passar pela validação;
- L4: a mistura e a ordem dos arquivos, o `f `, o limite, a junção sem repetir caminho, a abreviação com `~`, o descarte de uma geração antiga, os buffers da IPC do Everything e o escape do termo para cada fonte;
- L6: as ações por tipo de resultado, o Fixar e Desafixar e o Ctrl+C com e sem seleção (seção 8f);
- L7: a regra de que janela conta, a mistura e a ordem com apps e arquivos, o `j `, a exclusão do Dockyard e as ações de janela (seção 8g);
- L5: o reconhecimento de um caminho, a expansão de `~` e `%VAR%`, a pasta e o filtro, a ordem e os ocultos, o Tab, o Backspace e o descarte de uma leitura antiga (seção 8e);
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

## 8e. Como o L5 ficou

**Navegar por caminho, como no Ulauncher** (`domain/launcher_paths.rs`, puro e testado; `windows/launcher/paths.rs`, a leitura da pasta):

- **Quando liga** (`launcher_paths::root_len`): a consulta (sem os espaços do começo) começa com uma letra de unidade e `:\` (`C:\`, `d:\`), com `~\` (o perfil, `USERPROFILE`), com `%NOME%\` (uma variável de ambiente do usuário, pelo ambiente do processo; o nome só com letras, números, `_` e parênteses, como `%ProgramFiles(x86)%`, até 64) ou com `\\servidor\compartilhamento\` (com a `\` do fim; `\\.\` e `\\?\`, dos dispositivos, não valem). `C:`, `~`, `%APPDATA%` e `\\servidor\compartilhamento` sem a `\` ainda são digitação comum.
- **Nesse modo, só o caminho responde:** nem apps, nem Configurações, nem conta, nem busca na web, nem a busca de arquivos do L4.
- **A pasta e o filtro:** tudo até a última `\` é a pasta digitada; o resto é o filtro. `~` e `%NOME%` são expandidos; `.`, `..` e `\` repetida são resolvidos sem subir acima da unidade ou do compartilhamento (`C:\Users\.\ana\..\bob\` lê `C:\Users\bob\`). Uma variável que não existe ou que não é um caminho completo dá "Pasta não encontrada", sem ler nada.
- **O que lista** (`launcher_paths::list`): as pastas primeiro, depois os arquivos, até **50**, com a lista rolando como no L4. Sem filtro, em ordem alfabética, sem diferenciar maiúsculas nem acentos. Com filtro, só os nomes que ele casa pelas faixas dos apps (seção 5; para um arquivo, também o nome sem a extensão) e, dentro de cada grupo, **a melhor faixa primeiro e depois a ordem alfabética**: com o filtro `rel`, "relatorio.txt" (nome exato sem a extensão) vem antes de "meu relatorio.txt" (uma palavra), em vez da ordem alfabética pura, que poria uma letra espalhada (faixa 6) antes de um nome exato. Sem filtro, todos estão na mesma faixa e a ordem é a alfabética.
- **Ocultos:** os itens com o atributo oculto ou de sistema, e os nomes que começam com `.` (como `.git`), ficam de fora, a não ser que o filtro comece com `.`.
- **A linha:** o nome como título e "Pasta" ou "Arquivo" como subtítulo (a pasta é a digitada, igual em todas as linhas). O ícone vem do auxiliar, como no L4; num compartilhamento de rede a linha fica com o glifo, para o shell não ir à rede e segurar a fila de ícones do auxiliar.
- **Tab** completa a linha selecionada no campo: a pasta digitada mais o nome, e `\` numa pasta, que entra nela e a lista (`~\Doc` → `~\Documents\`). Num arquivo ou pasta da busca do L4 ou dos recentes, Tab põe o caminho completo, o que entra no modo de caminho. **Antes do L5, o Tab não fazia nada no lançador** (era engolido para não apitar e o foco nunca saía do campo); continua sem mover o foco, que fica no campo como antes. A lista nunca teve foco: o leitor de tela acompanha a seleção pela UI Automation (seção 7). Shift+Tab não faz nada. Depois do Tab, o cursor vai para o fim e o anúncio "N resultados" sai de novo.
- **Backspace** com o cursor no fim, sem seleção, e o campo terminando em `\` apaga a pasta inteira (`launcher_paths::up`: `C:\Users\ana\` → `C:\Users\`). Na raiz (`C:\`, `~\`, `%APPDATA%\`, `\\servidor\compartilhamento\`) apaga um caractere, como sempre. É tratado no caractere (`WM_CHAR`), onde a caixa de texto apaga.
- **Enter** abre o arquivo pelo programa padrão ou a pasta no Explorador, pela ação do L4 (`Action::OpenFile`). A validação passou a aceitar também um caminho num compartilhamento (`launcher_files::is_valid_unc`, usada por `is_openable` na ação, em `apps::open_path` e nas chaves de uso); a busca de arquivos do L4 continua só com unidades. **Uso:** abrir conta como `file:<caminho>`, como no L4, e aparece nos recentes.
- **Erros:** uma pasta que não existe (ou é um arquivo) mostra "Pasta não encontrada"; uma sem permissão, "Sem acesso"; uma que não responde em **2 s** (um compartilhamento fora do ar), "A pasta não respondeu". Uma pasta vazia ou um filtro sem nada mostram "Nenhum resultado". Nada trava.

**Onde roda e o descarte:**

- A pasta é lida com `std::fs::read_dir` (sem shell, sem COM) numa thread `launcher-folder` do processo principal, uma por pasta; os atributos vêm com a listagem, sem outra ida ao disco. A leitura para em **20 000** itens ou em 2 s, e devolve o que leu.
- **Uma vez por pasta** (`launcher_paths::Browse`): enquanto só o filtro muda, o que foi lido é filtrado a cada tecla, na thread da UI, sem ler de novo. A pasta é lida de novo quando a pasta digitada muda (sem diferenciar maiúsculas), e a cada abertura do lançador.
- **A geração:** cada leitura leva um número; uma resposta de uma pasta que não é mais a digitada, de depois do tempo limite ou de depois de o lançador fechar é descartada. O temporizador de 2 s na thread da UI cobre a pasta que trava dentro do `read_dir` (a thread fica presa até o Windows desistir, mas a resposta não é usada).

**Testes de unidade:** o reconhecimento do modo (`C:\`, `~\`, `%APPDATA%\`, UNC e o que não é caminho), a expansão de `~` e `%VAR%`, a separação entre pasta e filtro com `.` e `..`, a ordem pastas → arquivos, o filtro pelas faixas, os ocultos, o Tab, o Backspace, o descarte da geração antiga e do tempo limite, a validação do UNC e a leitura de uma pasta de verdade (temporária) com os erros.

**Não verificado (só ao vivo):** o Tab e o Backspace no teclado de verdade (a caixa de texto do Windows, o cursor no fim depois do Tab, o IME); a lista de `C:\Windows\System32` (uns 5 000 itens) filtrando a cada tecla sem atraso; um compartilhamento de rede de verdade, e um fora do ar mostrando "A pasta não respondeu" sem travar; "Sem acesso" numa pasta protegida (`C:\System Volume Information`); os ícones das linhas; o Narrador lendo "Pasta" e "Arquivo" e o anúncio depois do Tab; a memória com o lançador aberto navegando.

## 8f. Como o L6 ficou

**As ações por tipo** (`domain/launcher_actions.rs`, puro e testado), na ordem do menu:

| Resultado | Ações |
|---|---|
| App | Abrir · Abrir como administrador · **Fixar no dock** ou **Desafixar do dock** · Mostrar no Explorador (só app clássico com arquivo) |
| Arquivo | Abrir · Abrir como administrador (só `.exe`, `.com`, `.bat`, `.cmd`, `.msi`, `.msc` e `.lnk`) · Mostrar no Explorador · Copiar caminho |
| Pasta | Abrir · Mostrar no Explorador · Copiar caminho |
| Página de Configurações, busca na web | Abrir |
| Conta | Copiar resultado |

- **Uma ação só, sem menu:** página, busca na web e conta têm uma ação, a mesma do Enter. Shift+Enter faz essa ação direto; a tecla Menu, Shift+F10 e o botão direito não abrem nada nessas linhas (a tecla Menu abre o menu do próprio campo de texto, como antes). Um menu de uma linha só seria um passo a mais para o mesmo Enter.
- **App da Loja ou clássico:** o id de um app empacotado (Loja e afins) é `FamíliaDoPacote!App`; ele nunca tem "Mostrar no Explorador". Para um clássico, o lançador pergunta ao shell o arquivo do app (`apps::target_of`, o mesmo do menu do dock) **antes de abrir o menu**, numa thread `launcher-app-file`, e guarda a resposta até liberar a janela; sem arquivo (um atalho para uma URL, um app do sistema), o item fica de fora. A pergunta só acontece no primeiro menu de cada app.
- **"Abrir como administrador" aparece em todo app,** inclusive nos da Loja, como o autor decidiu: alguns empacotados aceitam (o Terminal do Windows); um que não aceita mostra "Não foi possível abrir".
- **A lista fechada cresceu** (`launcher::Action`): `OpenAppAsAdmin` (id válido), `OpenFileAsAdmin` (caminho que o lançador abre, de um tipo que o Windows eleva), `Reveal` e `CopyPath` (caminho que o lançador abre, de unidade ou compartilhamento; o limite de 64 caracteres é só da conta), `PinApp` e `UnpinApp` (id válido). `launcher_actions::action` monta a ação de um item a partir da ação do Enter e passa por `is_allowed`; um item que não se aplica não vira ação. `apps::open_app_as_admin`, `open_path_as_admin` e `reveal_path` conferem de novo.

**Abrir como administrador** (Ctrl+Shift+Enter ou o item do menu): `ShellExecuteExW` com o verbo `runas`, numa thread `launcher-admin`, com a janela do lançador como dona da pergunta do UAC. Um app vai pelo item da pasta Aplicativos (`shell:AppsFolder\<id>`), como o "Executar como administrador" do Iniciar; se o shell recusar e o app clássico tiver arquivo, vai pelo arquivo, com os argumentos do atalho. **Cancelar o UAC não é erro** (`ERROR_CANCELLED`): o lançador continua aberto, com o foco na caixa, sem mensagem. Outra falha mostra "Não foi possível abrir" na linha. Enquanto o Windows pergunta, o lançador não fecha ao perder o foco. Abrir como administrador conta no uso, como abrir. Ctrl+Shift+Enter num resultado sem essa ação (um documento, uma pasta, uma página) não faz nada.

**Fixar e desafixar:** pelo mesmo caminho do painel (`commands::apps::pin` e `unpin`, por `change`, que avisa as janelas, os espaços de trabalho e o dock nativo), no espaço de trabalho em uso. "Desafixar do dock" aparece quando o espaço em uso já tem o app (`commands::apps::pinned_here`, sem diferenciar maiúsculas no id); fixado só em outro espaço, o item é "Fixar no dock" e o põe neste também, como o painel faz. O nome é o do catálogo, que veio do shell.

**Mostrar no Explorador:** `explorer /select,"<caminho>"`, como o menu do dock (`apps::reveal`), numa thread `launcher-open`. Abre a pasta de cima com o item selecionado; numa pasta, seleciona a própria pasta; numa unidade, abre a unidade.

**Copiar:** Ctrl+C, sem seleção no campo (`EM_GETSEL` com o início igual ao fim), copia o caminho do arquivo ou da pasta (a conta copia o resultado, como o Enter). É tratado no caractere (`WM_CHAR` 0x03), onde a caixa de texto copia; com seleção, a caixa copia o texto. Copiar fecha o lançador e diz "Caminho copiado" ao leitor de tela. Num app, numa página ou numa busca, Ctrl+C sem seleção continua sendo o da caixa (não copia nada).

**O menu:** o próprio do app (`windows::menu::show_from`, o do dock), com a janela do lançador como **dona**: fica acima dela e, ao fechar, o Windows devolve a ativação ao lançador. Pelo teclado (Shift+Enter, tecla Menu, Shift+F10), abre embaixo da linha selecionada, na ponta direita, com o primeiro item aceso; pelo botão direito, no ponteiro, depois de selecionar a linha. Cada item mostra o atalho que faz o mesmo sem o menu (Enter, Ctrl+Shift+Enter, Ctrl+C).

- **O laço do menu fora do empréstimo:** o menu roda um laço de mensagens próprio até a escolha. Ele abre por uma mensagem da janela (`WM_ACTION_MENU`), fora do `RefCell` do lançador, para que, enquanto está aberto, o lançador continue pintando e recebendo os arquivos e os ícones (com o lançador emprestado, esse trabalho se perderia).
- **Não fechar ao perder o foco:** enquanto o menu está aberto, o `WM_ACTIVATE` inativo do lançador é ignorado (`HOLD`), e o atalho global espera. Fechado o menu com Esc, o lançador pega o teclado de volta; fechado com um clique em outro programa (o primeiro plano não é mais do Dockyard), o lançador fecha, como fecharia sem o menu.
- **Escolher fecha:** cada ação fecha o lançador, como abrir, ou o deixa aberto dizendo "Não foi possível abrir" na linha.

**Testes de unidade:** as ações por tipo (app da Loja sem "Mostrar no Explorador", clássico com; arquivo comum sem "Abrir como administrador", executável e atalho com; pasta; página, busca e conta sem menu), "Fixar" virando "Desafixar" e a ação de cada um, o Ctrl+C com e sem seleção, a ação que não se aplica, as novas ações da lista fechada (`every_action_is_checked`) e `pinned_here` antes e depois de fixar e desafixar.

**Não verificado (só ao vivo):** o menu na tela, embaixo da linha e no ponteiro, em claro, escuro e alto contraste; o lançador continuar aberto com o menu e fechar com um clique em outro programa; o UAC de verdade (aceitar abre elevado; cancelar deixa o lançador aberto, com o foco); `runas` num app da Loja que não aceita elevação e num app clássico pela pasta Aplicativos; o Explorador abrindo com o item selecionado; fixar e desafixar aparecendo no dock; o Ctrl+C, a tecla Menu e Shift+F10 com o teclado de verdade; o Narrador lendo o menu e "Caminho copiado".

## 8g. Como o L7 ficou

**As janelas abertas** (`domain/launcher_windows.rs`, puro e testado; `windows/launcher/open_windows.rs`, a leitura):

- **Que janela conta:** a mesma regra do dock, a da barra de tarefas: visível, sem dona, sem `WS_EX_TOOLWINDOW` nem `WS_EX_NOACTIVATE` (a não ser com `WS_EX_APPWINDOW`, que pede o botão) e não "cloaked" (outra área de trabalho virtual, app da Loja suspenso), com título. A regra saiu de `windows/openwin.rs` para `domain::open_apps::has_button`, pura e testada com dados sintéticos, e o dock e o lançador leem pela mesma `openwin::enumerate`, que já devolvia a ordem Z (a do topo primeiro).
- **Fora da lista:** o próprio lançador e todas as janelas do Dockyard (o processo dele: o dock, Configurações) e de outro processo do mesmo programa (o auxiliar de ícones, uma segunda cópia), comparando o `.exe` sem diferenciar maiúsculas (`launcher_windows::is_ours`).
- **A linha:** o título da janela como título; "Janela · <nome do app>" como subtítulo ("Janela", só, quando o app não tem nome). O nome e o ícone vêm do que o dock já sabe (`open_apps::programs_of_windows`, sem perguntar ao shell): o app fixado (o nome e o id do dock) ou o aberto não fixado (o nome e o id que o dock descreveu, inclusive o perfil do Chrome). Se o dock não mostra o programa (a opção "apps abertos" desligada), o app da pasta Aplicativos cujo id é o AppID da janela; por fim, o `.exe` e o nome dele sem a extensão. O ícone é lido pelo mesmo caminho do app (`Request::App` do auxiliar, ou o pacote de ícones); enquanto não chega, o glifo de janela.
- **A mistura:** com qualquer consulta, as janelas casam pelo **título** e pelo **nome do app**, pelas mesmas faixas dos apps (seção 5), a melhor das duas; dentro da faixa, na ordem Z. Entram numa faixa própria **depois dos apps, das páginas e da conta e antes dos arquivos** (`launcher_windows::mix`), até **8**. A busca na web padrão ("quando nada casa") só aparece se não houver app, janela nem arquivo. Uma conta ou uma busca na web por palavra-chave vêm sozinhas, como no L4, e `f termo` mostra só arquivos.
- **A palavra-chave `j `** ("janelas"): mostra **só** as janelas, até 30; `j ` sem nada depois lista **todas, na ordem Z** (a usada por último primeiro). `j` sem espaço é digitação comum. **Por que não `w`:** `w` é a Wikipédia nas palavras-chave padrão da web. **`j` não colide** com as padrão (`g`, `yt`, `w`, `gh`) nem com o `f` dos arquivos (um teste garante); se o usuário criar uma palavra-chave da web `j`, **a dele vence**, como com o `f`.
- **Ler:** as janelas são lidas na thread da UI, ao abrir o lançador e a cada tecla (uma janela aberta ou fechada no meio aparece ou some). **Medido** nesta máquina (`reading_the_windows_is_cheap`, ignorado no `cargo test`): 0,25 ms de mediana com 2 janelas; a primeira leitura do processo, 4,6 ms (as DLLs do shell carregando). O custo é um `EnumWindows` e, por janela com botão, o título, o `.exe` (`OpenProcess`) e o AppID (`SHGetPropertyStoreForWindow`), da ordem de 0,1 ms por janela. Uma leitura acima de 5 ms vai para o log, para medir ao vivo.

**As ações** (`launcher_actions`, no menu do L6):

| Resultado | Ações |
|---|---|
| Janela | **Alternar para a janela** (Enter) · **Fechar janela** · Mostrar no Explorador (o `.exe` do app; fora para app da Loja, cuja pasta `WindowsApps` é fechada ao usuário, e para o `ApplicationFrameHost.exe` que emoldura um) |

- **A lista fechada cresceu:** `Action::FocusWindow(hwnd)` e `Action::CloseWindow(hwnd)`, com o handle diferente de zero em `is_allowed`. Antes de agir, o lançador confere de novo que o handle ainda é uma janela com botão de outro processo (`open_windows::still_listed`): o Windows reaproveita handles, e uma janela que fechou no meio mostra "Não foi possível abrir" na linha.
- **Alternar** usa a mesma função do dock (`openwin::focus`): restaura se estiver minimizada, chama `SetForegroundWindow` e, se o Windows recusar, o truque de `AttachThreadInput` da seção 3. Como o lançador está em primeiro plano, o Windows deixa ele passar a frente. Depois avisa o dock (`open_apps::wake`), como o clique do dock.
- **Fechar janela** manda `WM_CLOSE` pela função do dock (`openwin::close`), como o botão de fechar: o programa pode perguntar se salva ou recusar. **Nunca** mata o processo. O lançador fecha, como em toda ação, e a pergunta do programa fica visível.
- **Espaços de trabalho do Dockyard:** o dock não tem um "clicar numa janela de outro espaço": uma janela de outro espaço no mesmo monitor está **estacionada** (`window_spaces`, escondida com `SW_HIDE`), fora da barra e do Alt+Tab, e o dock só mostra as do espaço em uso (`counts`). O lançador segue isso: lista só as janelas visíveis, então as estacionadas ficam de fora, e uma janela de outro espaço **em outro monitor** (visível, porque aquele monitor mostra o espaço dela) é alternada normalmente, sem trocar de espaço.
- **Uso:** alternar para uma janela **não conta** no uso e janela **não aparece nos recentes**. Uma janela é efêmera: o handle muda a cada abertura do programa (e o Windows o reaproveita para outra janela), então a chave (`window:<handle>`) não teria sentido na abertura seguinte do lançador; e o que se usa muito é o **app**, que já conta quando é aberto. Fechar e mostrar no Explorador também não contam.
- **Acessibilidade:** o item da UI Automation tem o tipo "Janela"; o Narrador lê "Relatório.docx - Word, Janela, 3 de 9".

**Testes de unidade:** a regra de que janela conta (visível, cloaked, tool window, `WS_EX_NOACTIVATE`, com dona, `WS_EX_APPWINDOW`), a busca pelo título e pelo app, a faixa vencendo a ordem Z, a mistura entre apps e arquivos e a busca na web só quando nada casa, o `j ` (todas na ordem Z, com termo, o limite, `j` sem espaço, a palavra-chave do usuário vencendo, sem colidir com as padrão), a exclusão do Dockyard (o processo e o `.exe`, sem diferenciar maiúsculas), a descrição do programa (dock, catálogo, `.exe`), o Mostrar no Explorador fora dos apps da Loja e as ações de janela na lista fechada.

**Não verificado (só ao vivo):** Enter trazendo para a frente uma janela comum, uma minimizada e uma de programa elevado (o Windows pode barrar o foco de uma janela de administrador vinda de um processo comum); Fechar janela num programa com documento não salvo (a pergunta aparecendo); o tempo de leitura com 20–30 janelas abertas (a meta de 5 ms); o ícone e o nome de cada app (Chrome com perfis, Explorador, apps da Loja via `ApplicationFrameHost`); uma janela de outro espaço em outro monitor; o Narrador lendo "Janela".

## 8h. Área de transferência

**Decisões do autor (2026-10-30):**

- **A fonte é o histórico do próprio Windows** (Win+V), pelo WinRT `Windows.ApplicationModel.DataTransfer.Clipboard`: `IsHistoryEnabled`, `GetHistoryItemsAsync` e `SetHistoryItemAsContent`.
- **O Dockyard não guarda nada:** nem em disco, nem em cache entre aberturas.
- **Com o histórico do Windows desligado,** aparece uma linha só, "Histórico da área de transferência desligado", que ao Enter abre `ms-settings:clipboard`.

**Como ficou** (`domain/launcher_clipboard.rs`, puro e testado; `windows/launcher/clipboard.rs`, a leitura e as linhas):

- **A palavra-chave `cb `** ("clipboard"), e só ela: o histórico pode ter senhas e outras coisas sensíveis, e nunca aparece misturado numa busca comum. `cb` sem espaço é digitação comum. **Por que `cb` e não `ac`:** as duas são curtas e nenhuma colide com as padrão (`g`, `yt`, `w`, `gh`, `f`, `j`; um teste garante), mas `cb` segue o costume das outras, abreviações em inglês como `yt` e `gh`; `ac` ("área de cópia") não é o nome que o Windows usa ("área de transferência" daria `at`). Se o usuário criar uma palavra-chave da web `cb`, **a dele vence**, como com o `f` e o `j`.
- **Nesse modo, só o histórico responde:** nem apps, nem Configurações, nem conta, nem busca na web, nem janelas, nem arquivos.
- **Sem termo,** os itens mais recentes, até **25** (o que o Windows guarda). **Com termo,** os itens cujo texto tem cada palavra do termo em qualquer lugar, sem diferenciar maiúsculas nem acentos, na ordem do mais recente. O filtro olha os primeiros 4 096 caracteres de cada texto.
- **Só itens de texto** (os que têm o formato `Text`). Imagens, arquivos e outros formatos ficam **de fora nesta versão**; a leitura só conta quantos foram pulados.
- **A linha:** a primeira linha com algum texto, com espaços e tabulações juntados, cortada em **80 caracteres** (o último vira `…`); um texto só de espaços aparece como "Texto em branco". O subtítulo: "Área de transferência · há 5 min" ("agora", "há N min", "há N h", "há 1 dia", "há N dias"). O glifo de colar no lugar do ícone.
- **Enter** põe o item como o conteúdo atual da área de transferência (`SetHistoryItemAsContent`, pelo id do item, lido de novo do histórico numa thread `launcher-clipboard`), fecha o lançador e anuncia "Copiado" ao leitor de tela. **Não cola** no programa: colar exigiria mandar Ctrl+V com `SendInput`, que o projeto evita. Um item apagado do histórico no meio mostra "Não foi possível abrir".
- **As ações (L6):** **Copiar** (Enter, o padrão) · **Abrir como link**, só quando o texto inteiro (sem os espaços das pontas) é um endereço `https://` que o lançador abre (`launcher_clipboard::link`, a mesma checagem das buscas na web). Sem link, a linha tem uma ação só: Shift+Enter copia, sem menu. Ctrl+C sem seleção no campo copia o item, como o Enter.
- **A lista fechada cresceu:** `Action::CopyClip(id)`, com o id do item do Windows (não vazio, até 128 caracteres, sem controle). **O texto copiado nunca entra na ação.** "Abrir como link" vira `Action::OpenUri`, checado como sempre.
- **Uso e recentes:** um item **não conta** no uso e não aparece nos recentes. A linha do histórico desligado conta como a página "Área de transferência" das Configurações do Windows, que ela abre.

**Onde roda e o descarte:**

- A leitura é do WinRT, assíncrona, e roda numa thread `launcher-clipboard` do processo principal, esperando a operação (`join`). Ela começa quando a consulta entra em `cb ` e é feita **uma vez** enquanto a consulta fica em `cb `: o termo só filtra o que foi lido, na thread da UI, a cada tecla.
- **A geração e o tempo limite,** como no L4 e no L5: cada leitura leva um número; uma resposta depois de **1,5 s**, depois de a consulta sair de `cb ` ou de o lançador fechar é descartada. Sem resposta a tempo, "Não foi possível ler a área de transferência" (o mesmo texto de quando o Windows recusa).
- **Nada fica:** sair de `cb ` ou fechar o lançador esquece o que foi lido. Nada vai para o disco.
- **Por que não no auxiliar:** a API do WinRT da área de transferência pesa pouco no processo (medido abaixo), e o Windows pode recusar o histórico a um processo que não tem o primeiro plano: o lançador aberto tem, o auxiliar nunca tem janela à frente.

**Privacidade:** nada do conteúdo vai para o log, nem com o trace ligado. O log diz só quantos itens de texto e de outros tipos o histórico tinha, o tempo, e o código de status quando o Windows recusa. Duas guardas: o texto de um item é um `ClipText`, que não tem `Display` (um `log::info!("{}", texto)` não compila) e cujo `Debug` escreve só o tamanho (`ClipText(23 chars)`), com um teste; e um teste que lê as linhas de `log::` de `windows/launcher/clipboard.rs` e falha se alguma levar o texto, a linha, o nome ou o id de um item.

**Testes de unidade:** a palavra-chave (só `cb ` com espaço, `cb` sozinho e as outras consultas sem histórico, a palavra-chave do usuário vencendo) e as colisões com as padrão, o `f` e o `j`; o filtro (sem acento, sem maiúsculas, todas as palavras, a ordem do mais recente, o limite de 25); o corte da primeira linha (linhas em branco, tabulações, 80 caracteres por caractere e não por byte); o "há N min" (agora, minutos, horas, dias, relógio atrasado); a conversão do `DateTime` do WinRT; o histórico desligado, a resposta atrasada e a de antes de um descarte; a URL (só um `https://` sozinho; `http://`, texto em volta, duas linhas, `javascript:` e endereço longo demais ficam de fora); as ações (Copiar e Abrir como link, o Ctrl+C, a ação na lista fechada); e as duas guardas de privacidade.

**A medir:** o teste `clipboard_history_answers`, ignorado no `cargo test` e rodado à mão com `cargo test clipboard_history_answers -- --ignored --nocapture`, imprime só a contagem e o tamanho dos itens (nunca o conteúdo), o tempo das duas leituras e a memória privada que o WinRT somou ao processo. Ele ainda não rodou: no dia do PR o disco da máquina estava cheio e o build de teste não coube. Se a memória passar de 1–2 MB, a leitura vai para o auxiliar, como no L4, com o cuidado do primeiro plano acima.

**Não verificado (só ao vivo):** a leitura com o lançador em primeiro plano (o teste roda de um terminal, que é quem tem o primeiro plano); o Enter pondo o item de volta e o Ctrl+V em outro programa colando-o; "Abrir como link" no navegador; a linha do histórico desligado abrindo a página; o tempo e a memória com o lançador aberto; o Narrador lendo "Área de transferência" e "Copiado".

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
