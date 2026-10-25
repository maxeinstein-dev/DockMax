# Interface do PrimoDock, observada em uso

Capturas de 2026-10-25, do PrimoDock 0.7.0-beta em funcionamento na máquina do autor (um monitor de 1920 × 1200, tema Flutuante, pacote Pixel System): capturas de tela do agente (mouse e a acessibilidade do Windows, que expõe os menus WPF com nomes e posições), capturas do autor e três gravações de tela do autor (dock e bandeja, 12 s; passar o mouse, 28 s; a janela de configurações inteira, 70 s), lidas quadro a quadro. **As imagens e os vídeos ficam fora do repositório** (`.local/primodock-capturas/`, ignorada pelo git); aqui só descrições, textos da interface e medidas.

## O dock parado (tema Flutuante)

Da esquerda para a direita:

- **Bolinhas de espaço de trabalho**: três pontos na vertical, um por espaço, na cor dele (azul, laranja, verde); o ativo é mais forte.
- Um **separador** fino, vertical.
- **Botão Iniciar** (o lançador; abre o Iniciar do Windows ou os apps do PrimoDock, conforme a configuração), depois os **apps fixados** do espaço. No tema Flutuante cada ícone é uma placa do pacote (Pixel System: placa arredondada verde-água com a arte em pixel), sem barra por trás.
- Os **widgets** do espaço, mais largos: "Atividade do sistema" (2 ícones de largura, fundo azul, barras de CPU e RAM com porcentagem em fonte monoespaçada) e "Atividade de rede" (2 ícones, verde-água, taxas de descida e subida e um gráfico de linha).
- Separador; os **apps abertos** que não estão fixados (Claude e VS Code), com o indicador embaixo.
- Separador; o **status do sistema** em duas linhas de ícones pequenos (bateria, Wi-Fi, volume; Segurança do Windows e Docker, que estão "no Dock" pela configuração da bandeja, e a seta da bandeja) e o **relógio** em duas linhas (hora em negrito, data).
- O **indicador** de app aberto é um ponto branco pequeno embaixo do ícone (o padrão `dot`).
- O **mascote** anda sobre o dock, por cima das janelas: corre de um lado para o outro quando o computador está ocupado e se deita para dormir (com "Z" saindo) quando está tranquilo.

Ícones de 52 px; a janela do dock tem 1189 × 175 px (bem maior que a barra, para caber a ampliação e o rótulo), e a faixa reservada é de uns 71 px.

## Os temas no próprio dock

Vistos numa gravação do autor trocando de tema na aba Aparência, com o mesmo dock (Iniciar, cinco apps, dois widgets, três apps abertos, bandeja e relógio). Em todos, os ícones, os widgets, as bolinhas e o mascote ficam iguais: o tema muda só o que está **atrás** e **em volta** deles.

| Tema | Barra |
|---|---|
| Flutuante | sem barra: os ícones soltos sobre o papel de parede |
| Nativo | retângulo cinza-escuro translúcido, borda clara fina, cantos médios (uns 12 px) |
| Liquid Glass | pílula (cantos inteiros) de vidro escuro translúcido, com borda branca mais forte e um brilho interno no topo |
| Meia-noite | retângulo quase preto, borda branca fina, cantos pequenos |
| Rocket | azul-marinho escuro, borda e brilho externo azuis, cantos médios |
| Prateleira Aqua | sem barra: uma **prateleira prateada em perspectiva** (um trapézio mais largo embaixo) onde os ícones se apoiam; o relógio fica por cima dela |
| Neon | pílula preta com a borda em **degradê ciano para magenta** e brilho das duas cores |
| Canarinho | pílula em degradê amarelo para verde |
| Papel | pílula **branca opaca**; o relógio e os ícones da bandeja passam a escuros, e as bolinhas ficam em tons pastel |
| Halloween | roxo-escuro com borda laranja, **abóboras** enfileiradas na borda de cima e **teias de aranha** nos dois cantos |
| Noite Assombrada | preto com uma borda **verde-neon escorrendo** (gosma) e **fantasminhas** flutuando acima |
| Candy Corn | creme com a borda pontilhada de doces coloridos e doces e abóboras por cima |
| Natal | verde-escuro, **capa de neve** na borda de cima, um **cordão de luzes coloridas** acima e flocos caindo |
| Noite de neve | azul-marinho com capa de neve, luzes brancas e flocos |
| Candy Cane | branco com a borda **listrada em vermelho e branco** (bengala de doce), luzes e neve |

- Os sazonais têm **enfeites que se mexem** (flocos caindo, luzes piscando, fantasmas flutuando); a opção "Economia de bateria" da aba Geral os para.
- Em "Personalizar este tema", os sazonais trocam as opções de borda por **"Top color"** (a cor de cima, com uma paleta de cores; o rótulo aparece em inglês mesmo com a interface em português).
- Trocar de tema recria o dock: ele some e volta em menos de um segundo.

## As quatro bordas

Vistas na mesma gravação, trocando a posição com o dock carregado:

- **No topo**: a mesma ordem, da esquerda para a direita; o indicador de app aberto passa para **cima** do ícone (do lado da borda da tela) e a seta da bandeja aponta para baixo (∨).
- **Esquerda**: o dock vira uma **coluna**. De cima para baixo: as bolinhas de espaço de trabalho (agora **em linha**), separador, Iniciar e os apps, os widgets **girados 90°** (o texto e o gráfico na vertical), os apps abertos, e no pé a bandeja em duas colunas, a seta apontando para a direita (›) e o relógio com a data em duas linhas pequenas. O indicador fica do lado esquerdo do ícone.
- **Direita**: o **espelho girado** da esquerda, e não só refletido: o relógio e a bandeja ficam **no topo** e o Iniciar e as bolinhas no **pé**, com a seta apontando para a esquerda (‹). O indicador fica do lado direito.
- Em todas, a faixa reservada (barra de aplicativos) empurra a área de trabalho: os ícones da área de trabalho do Windows se rearrumam para longe do dock.

## Ocultar automaticamente

Com "Ocultar automaticamente" ligado (pelo menu do dock), o dock **some esmaecendo** (a opacidade cai até zero, sem deslizar) quando o ponteiro sai dele, e **volta esmaecendo** quando o ponteiro chega à borda da tela, depois do atraso de revelação (0,5 s).

## Ao passar o mouse

- **Rótulo**: uma pílula escura pequena aparece acima do item com o nome: o do app fixado ("FileZilla", "WhatsApp", "Terminal", "Google Chrome"), o do widget ("Atividade do sistema", "Atividade de rede"), o nome do executável para um app aberto não fixado ("Code.exe") e "Ícones de outros apps" sobre a seta da bandeja.
- **Escala** (o padrão, 1,10×): só o ícone sob o mouse cresce. **Ampliação**: o ícone cresce bem mais e os vizinhos crescem um pouco, como no macOS; nas gravações, o ícone ampliado chega a cerca de 1,4× e sobe acima da linha da barra.
- **Prévia de janela**: sobre um app aberto, depois do atraso (0,2 s), um cartão escuro acima do ícone com a miniatura ao vivo da janela, o título embaixo e um botão de fechar. Com várias janelas, uma miniatura por janela.

## Menus

Todos os menus são do mesmo jeito: cartão escuro com cantos arredondados (cerca de 8 px), itens de 37 px de altura com um **ícone de linha** à esquerda, o item sob o mouse **preenchido em coral** (a cor de destaque do app, perto de `#F0694B`) com o texto em branco, marcas de seleção (✓) também em coral, atalhos à direita em cinza, títulos de seção pequenos em cinza ("Workspaces") e separadores finos. Submenus abrem ao lado, com a mesma aparência. O menu principal tem cerca de 390 px de largura.

### Menu de um app (botão direito no ícone)

1. Nova janela
2. Remover do Dock
3. Mover para o workspace › (os outros espaços: Pessoal, Foco)
4. Fechar janela
5. Encerrar
6. Mostrar no Explorador de Arquivos
7. Ícone › Original · Pixel System (✓) · — · Escolher imagem… · Desenhar no IconStudio… · Galeria de ícones… · Abrir pasta de pacotes de ícones
8. — **Workspaces** — Trabalho (✓, `Ctrl+Alt+1`) · Pessoal (`Ctrl+Alt+2`) · Foco (`Ctrl+Alt+3`), cada um com a bolinha da cor
9. — Procurar atualizações… · Configurações… · Licença… · Encerrar PrimoDock

**Não mostra a Jump List** do programa (nem tarefas como "Nova janela anônima", nem recentes): "Nova janela" é do próprio PrimoDock. O DockMax já mostra a Jump List (spec 0006 §7), o que é uma vantagem.

### Menu do dock (botão direito no espaço vazio)

1. Adicionar espaçador pequeno
2. Adicionar espaçador grande
3. Widgets…
4. Colocar comida (alimenta o mascote)
5. — Efeito ao passar o mouse › Nenhum · Escala (✓) · Ampliação
6. Ocultar automaticamente
7. Mostrar apps abertos (✓)
8. — **Workspaces** (como acima)
9. — Procurar atualizações… · Configurações… · Licença… · Encerrar PrimoDock

### Menu de um widget (botão direito no widget)

1. Largura do tile › Um ícone · Dois ícones (✓) (só as larguras que o widget aceita)
2. Empilhar com › os outros widgets do espaço (por exemplo "Atividade do sistema")
3. Editar widget…
4. Remover do Dock
5. — **Workspaces** — e o rodapé, como acima

## Bandeja do sistema

- A seta (^) no fim do dock ("Ícones de outros apps") abre um **painel horizontal**: um cartão escuro arredondado com os ícones lado a lado, numa linha, logo acima da seta. O ícone sob o mouse ganha um quadrado claro por trás.
- **Botão direito** num ícone do painel abre o **menu do próprio programa** (no Claude: "Mostrar aplicativo", o plano, o uso e "Sair"), como na barra do Windows. O ícone do próprio PrimoDock mostra Configurações…, Licença… e Encerrar PrimoDock.
- **O painel fecha** assim que o programa abre o menu dele: o menu fica sozinho na tela, e para escolher outro ícone é preciso abrir o painel de novo. É o mesmo defeito que o DockMax tinha e corrigiu (o painel do DockMax fica aberto enquanto o menu do programa está aberto, ADR 0009 e Fase E). Confirmado numa gravação do autor.

## Fim do dock

- **Bateria, rede ou volume**: abre as **Configurações rápidas do próprio Windows** (Wi-Fi, Bluetooth, modo avião, brilho, volume, bateria, mídia tocando), ancoradas no canto, não um painel do PrimoDock.
- **Relógio**: abre o painel de **notificações e calendário do próprio Windows** (com o "Foco" de 30 minutos), ancorado no canto. O DockMax faz o mesmo com o calendário (ADR 0009).

## Espaços de trabalho

- Clicar numa bolinha (ou rolar a roda do mouse sobre o dock, ou `Ctrl+Alt+número`) troca o espaço: o dock passa a mostrar **só os apps e widgets daquele espaço** (no Foco: Iniciar, Bloco de notas, terminal e um app de tarefas; no Pessoal: Iniciar, Edge, Fotos, Paint, Loja e um gravador), e aparece por um instante uma **pílula escura acima das bolinhas** com a bolinha colorida e o nome ("● Foco").
- O status, a bandeja e o relógio continuam iguais em todos os espaços.

## Janela de configurações

Janela escura de uns 1220 × 850 px, sem a barra de título do Windows (só o X no canto), com o papel de parede borrado por trás (acrílico).

- **Barra lateral**: o ícone do app e "PrimoDock / Configurações" no topo, o campo **"Buscar nos ajustes"**, e as abas, cada uma com um **ícone colorido em placa**: Workspaces (azul), Widgets (roxo), Galeria de ícones (verde-água), IconStudio (laranja), Aparência (rosa), Geral (cinza), Licença (verde), Sobre (cinza-azulado). A aba ativa tem fundo mais claro e um **traço coral à esquerda**. No rodapé, um cartão do período de teste (fora do escopo do DockMax).
- **Cabeçalho de cada aba**: a placa colorida grande, o título e uma frase de apoio.
- **Seções**: um título pequeno em cinza e um cartão arredondado um pouco mais claro que o fundo, com linhas separadas por traços finos. Cada linha tem o nome em negrito, uma descrição em cinza embaixo e o controle à direita: **chave** (coral quando ligada), **controle segmentado** (a opção escolhida em fundo escuro), **controle deslizante** (trilho coral até o botão, com o valor em fonte monoespaçada à direita, "52 px", "0,2 s", "1,10×", "40%") ou um botão.
- **Escolhas visuais** (tema, posição, formato, efeito, animação, indicador, pet) são **cartões com miniatura**, em grade de 3 colunas; o escolhido tem a borda coral e o nome em negrito. A miniatura do escolhido **anima** o efeito (o ícone pula, pulsa ou treme; a lupa amplia).

### Aba Workspaces

- "Cada workspace tem seu próprio Dock e guarda suas próprias janelas. Troque pelas bolinhas na barra, pela roda do mouse sobre ela ou com Ctrl+Alt+número."
- **Fichas dos espaços** (bolinha, nome, ✓ no ativo) e **"+ Novo workspace"**.
- **Cartão do espaço ativo**: uma placa com o número do atalho, o nome, as teclas `Ctrl` `Alt` `1` desenhadas como teclas, "5 apps · 2 widgets", o selo "Ativo", e os botões duplicar e excluir.
- **"O Dock deste workspace"**: uma prévia do dock sobre o papel de parede; "Arraste um item para reordenar. Puxe para fora da barra para remover. Clique com o botão direito para renomear."
- **"Apps"**: "Clique duas vezes num app para adicioná-lo no fim, ou arraste para a barra acima para colocá-lo onde soltar." Campo "Buscar apps", botões **"Procurar…"** (escolher um executável) e **"Espaçador"**, e uma grade de 6 colunas com os ícones **originais** dos programas e o nome abaixo, cortado com reticências.
- **"Pastas e arquivos"**: "Depois dos apps, passando a linha. Uma pasta abre no Explorador de Arquivos." Vazio: "Nada aqui ainda — Adicione Downloads, a pasta de um projeto ou um arquivo que você abre todo dia", e os botões **"Adicionar pasta…"** e **"Adicionar arquivo…"**.

### Aba Widgets

"Coisas vivas dentro do Dock: um quadrado quando é um número, um cartão largo quando tem mais a dizer. Clique num para adicionar a este workspace." Campo **"Buscar widgets"** e o catálogo em cartões com a miniatura viva, por categoria:

- **Mídia e tempo**: Tocando agora · Calendário · Calendário · Dia · Relógio · Analógico · World Clock · Timer de foco · Pomodoro · Contagem regressiva · Progresso do tempo
- **Sistema**: Bateria · Estado · Bateria · Anel de porcentagem · Bateria · Só o ícone · Atividade do sistema · Barras · Atividade do sistema · Anéis · Atividade do sistema · Números · Atividade de rede · Atividade de rede · Gráfico · Layout do teclado
- **Ferramentas**: Beber água · Capturas de tela · Arquivos
- **IA**: Consumo do Claude · Consumo do Claude · Cartão · Uso do Codex · Uso do Codex · Cartão · Uso do OpenCode · Uso do OpenCode · Provedores
- **Notas e clima**: Post-it · Lista de tarefas · Tarefas · Contagem · Clima · Agora · Clima · Previsão

Cada variante (por exemplo "Atividade do sistema · Barras") é uma entrada do catálogo; a mesma entrada vira o mesmo widget com outro `style`.

Depois, **"Em uso"** ("Todos os widgets que você adicionou, e onde aparecem"): um cartão por widget com a miniatura, o nome e a lixeira, e as opções em linhas:

- **Largura do tile**: Um ícone · Dois ícones · Três ícones · Quatro ícones (só as que o widget aceita; a rede só tem Um e Dois);
- **Estilo**: Barras · Números · Anéis (na Atividade do sistema);
- **CPU** e **Memória**: chaves;
- **Mostrar nos workspaces**: fichas com a bolinha de cada espaço, várias escolhas.

Por fim, **"Espaçadores"** ("Espaços vazios entre os ícones do Dock"): "Este workspace — Clique direito num espaçador no Dock para removê-lo", com os botões **"Adicionar espaçador pequeno"** e **"Adicionar espaçador grande"**.

### Aba Galeria de ícones

"Pacotes prontos para vestir o seu Dock", com um selo amarelo "Incluído na sua licença". Um **cartão de destaque** grande (o pacote Primo, com uma faixa de ícones em perspectiva, "Destaque", a descrição, "Cobre 9 dos seus 9 apps" e o botão coral **Instalar**) e a grade dos outros pacotes: mosaico de 8 ícones de amostra, nome, "por PrimoDev · 207 ícones", "Cobre 9 dos seus 9 apps" e **Instalar** (ou "Em uso" no instalado). Os 14 pacotes estão em [pacotes-de-icones.md](pacotes-de-icones.md).

### Aba IconStudio (fora do escopo)

Fora do escopo do DockMax por decisão do autor. Registrado só para a referência: uma fila com os ícones do dock no topo; uma prévia grande do ícone escolhido; abas **Letras · Imagem · Ícone do app · Pacotes**; em Letras, o texto (até duas letras), a cor do texto, o peso (Regular · Semibold · Bold · Extrabold), o tamanho, o fundo (8 cores) e um rodapé "Nada para salvar ainda" com **"Usar este estilo em todos os apps"** e **"Salvar neste app"**; no topo, **"Restaurar ícones do sistema…"**.

### Aba Aparência

"Deixe o Dock com a sua cara." Em ordem:

1. **Tema** — "Visuais prontos para o Dock, como no RocketDock": Nativo · Liquid Glass · Meia-noite · Rocket · Prateleira Aqua · Neon · Canarinho · Papel · Flutuante, cada um com a miniatura do dock naquele tema.
2. **Sazonais** — "Temas de Halloween e de Natal": Halloween · Noite Assombrada · Candy Corn · Natal · Noite de neve · Candy Cane (luzes coloridas, neve caindo, capa de neve e moldura listrada).
3. **Personalizar este tema**: **Borda** (chave) · **Cantos** "O quanto a barra é arredondada" (deslizante, 40 % no Flutuante) · **Reflexos** "Uma imagem espelhada embaixo de cada ícone" (chave) · **Voltar ao original** "Descarta todas as mudanças neste tema" (botão **Restaurar**).
4. **Posição** — "Em qual borda o Dock fica. No topo, a barra de tarefas e o Iniciar continuam onde estão.": Embaixo · No topo · Esquerda · Direita.
5. **Tamanho**: **Tamanho dos ícones** (deslizante, 52 px) · **Espaço inferior** "Distância entre o Dock e a borda da tela" (deslizante, 2 px).
6. **Formato dos ícones** — "Os ícones do Windows vêm em todo tamanho e formato; o PrimoDock pode deixá-los parelhos": Placas arredondadas · Mesmo tamanho · Como vieram.
7. **Ícones dos apps** — "Troque os ícones de todos os apps por um pacote, ou mude um só pelo menu dele no Dock": Original · Pixel System (um cartão por pacote instalado); o botão **"Galeria de ícones…"** e os links **"Abrir pasta de pacotes de ícones"** e **"Recarregar pacotes"**; a chave **"Widgets seguem o pacote de ícones"** "Os widgets usam o pacote aplicado a todos os apps, quando ele tem estilo para widgets".
8. **Efeito ao passar o mouse** — "Como os ícones reagem ao passar o mouse": Nenhum · Escala · Ampliação; e **Escala ao passar o mouse** "Quanto o ícone cresce" (deslizante, 1,10×).
9. **Animação de abertura** — "Como o ícone se move enquanto o app abre": Pulo · Pulso · Tremor.
10. **Indicador** — "Marca embaixo dos apps abertos": Ponto · Traço; **Cor por workspace** "A cor de cada workspace marca os apps abertos, para você ver qual está ativo" (chave); **Cor do indicador**: "Padrão" e 17 bolinhas de cor em duas linhas.
11. **Pet** — **Mostrar o pet** "Um amiguinho que mora em cima do Dock. Corre quando o PC está ocupado e cochila quando está tranquilo." (chave); os bichos: Gato · Cachorro · Passarinho · Dragão · Dino · Guerreira · Guerreiro · Papai Noel · Fantasminha; **Pelagem**: quatro bolinhas de cor (as cores mudam com o bicho).
12. **Dock** — **Claro ou escuro** "O Dock e estas janelas seguem o modo de cor do Windows." (Automático).

### Aba Geral

"Como o PrimoDock se encaixa no Windows." Em ordem:

1. **Barra de tarefas do Windows** — três cartões com miniatura e explicação: **Deixar visível** "O Dock fica logo acima da barra de tarefas." · **Ocultar automaticamente** "Ela aparece ao encostar na borda da tela; o Iniciar do Windows a traz junto." · **Ocultar por completo** "O Iniciar abre sozinho, sobre o Dock. O relógio e os ícones da bandeja somem junto."
2. **Dock**:
   - **Mostrar apps abertos** — "Apps abertos que não estão fixados, depois de um separador"
   - **Seletor de workspace no Dock** — "Bolinhas coloridas no começo do Dock"
   - **Bateria, rede e volume** — "No fim do Dock enquanto a barra de tarefas está oculta; um clique abre as configurações rápidas do Windows"
   - **Relógio** — "A hora no fim do Dock enquanto a barra de tarefas está oculta, no formato regional do Windows; um clique abre o calendário"
   - **Mostrar a data** — "Abaixo da hora, como o relógio da barra de tarefas mostra"
   - **Ícones de outros apps** — "Com a barra de tarefas oculta por completo, uma seta no Dock mostra os ícones que os apps deixam na área de notificação"
   - **Ícones da bandeja no Dock** — "Ao lado de bateria, rede e volume, nas cores do próprio app. Arraste um do painel da seta ou escolha aqui; até 6." Uma linha por ícone da bandeja (ícone e nome, como o programa o chama) com o controle segmentado **Painel · Dock · Esconder**.
   - **Mostrar o botão Iniciar** — "Primeiro no Dock (arraste para onde quiser); útil com a barra de tarefas escondida"
   - **O botão Iniciar abre** — "Os apps do PrimoDock, ou o menu Iniciar do próprio Windows": **Apps do PrimoDock · Iniciar do Windows**
   - **Mostrar Downloads** — "Pasta Downloads em todo workspace"
   - **Mostrar Lixeira** — "Lixeira no fim do Dock"
3. **Ocultação**: **Ocultar e mostrar automaticamente** "O Dock aparece quando o ponteiro chega na borda dele" · **Manter as janelas fora do Dock** "Janelas maximizadas param acima dele em vez de ficar por baixo".
4. **Monitores**: **Em todos os monitores** "Cada monitor tem seu Dock e seu workspace; trocar em um não mexe nos outros".
5. **Prévias de janela**: **Prévia ao passar o mouse** "Pare o ponteiro sobre um app aberto para ver as janelas" · **Atraso da prévia** (deslizante, 0,2 s) · **Só com 2 ou mais janelas**.
6. **App**: **Economia de bateria** "Menos animação: sem pulo ao abrir, enfeites sazonais parados" · **Idioma** "O PrimoDock reabre para trocar": **Sistema · Português · English**.
7. **Inicialização**: **Abrir o PrimoDock quando o Windows iniciar** "Seu Dock já está lá assim que você entra."
8. **Atualizações**: "PrimoDock 0.7.0-beta — Você está na versão mais recente. Verificado às 14:59." e o botão **Verificar agora**.
9. **Atalhos** (só informativo): Ir para um workspace `Ctrl` `Alt` `1…9` · Próximo ou anterior — "Roda do mouse sobre o Dock" · Próximo widget de uma pilha — "ctrl + scroll, ou a roda lateral, sobre a pilha" · Ver as janelas de um app — "Pare o ponteiro sobre o ícone" · Fixar, mover ou fechar — "Botão direito num ícone".

### Aba Sobre

O ícone grande do app (placa coral com três pontos), "PrimoDock", "Versão 0.7.0-beta", "Inclui ManagedShell (Apache 2.0) e Velopack (MIT). Avisos em THIRD-PARTY-NOTICES.md.", o texto "Um Dock para cada coisa que você faz. Workspaces, prévias ao vivo e um pet na barra. Sem conta, sem analytics, sem rastreamento." e os botões **Enviar feedback** (coral), **Ver o tour**, **Site** e **Abrir pasta de dados**.

A aba Licença foi vista mas não registrada (fora do escopo).

## Notas para o DockMax

- **Linguagem visual a aproveitar como ideia**: menus escuros com ícone de linha em cada item e destaque preenchido; abas com ícone colorido em placa e traço de destaque; linhas com nome, descrição e controle à direita; controles segmentados para escolhas curtas; deslizantes com o valor em fonte monoespaçada; escolhas visuais como cartões com miniatura que anima o efeito escolhido; fichas com bolinha de cor para espaços.
- **Já melhor no DockMax**: o menu do app com a Jump List, e o painel da bandeja que não fecha enquanto o menu de um programa está aberto.
- **Ainda não tem no DockMax**: prévia de janela, apps abertos não fixados, espaços de trabalho e a troca por bolinhas e roda do mouse, espaçadores, "Pastas e arquivos" no dock, Downloads e Lixeira no dock, ícones da bandeja no próprio dock (Painel · Dock · Esconder), "Procurar…" para fixar um executável qualquer, ocultar automático, manter janelas fora do dock, dock em todos os monitores, temas sazonais, economia de bateria, idioma, abrir com o Windows, a busca nos ajustes e o mascote.
