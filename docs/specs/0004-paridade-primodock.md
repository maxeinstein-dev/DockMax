# Spec 0004 — Paridade com o PrimoDock

**Status:** rascunho para aprovação. Atualizada em 2026-10-22 com as capturas de tela do autor. Nenhum código de funcionalidade foi escrito.
**Origem:** pedido do autor de transformar o DockMax no PrimoDock, mantendo o nome, aceitando mudança de arquitetura.

## 1. Como o levantamento foi feito

- **Instalação local** (`%LOCALAPPDATA%\PrimoDock.App`, v0.7.0-beta): executável único de 208 MB, aplicativo .NET/WPF com SQLite, ManagedShell (Apache 2.0, hospeda a bandeja), Velopack (atualização) e Bouncy Castle (recibos de licença).
- **Dados do próprio usuário** (`%APPDATA%\PrimoDock`): `settings.json`, `workspaces.json`, `session.json`, `icons.json`, `pack.json` do pacote de ícones e o log. O esquema dessas configurações revela quase todas as opções do produto.
- **REA** (`rea-agents` 6.7.0, usado pelo CLI, sem registrar o MCP na configuração do Claude Code): o inspetor .NET recusa o executável porque o limite de memória dele é 128 MB. A extração de nomes por strings do binário só devolveu ruído e foi abandonada; o inventário abaixo vem da configuração e do log.
- **Capturas de tela do autor** (11 imagens do painel de configurações): preencheram as listas fechadas (temas, formatos, efeitos, animações, posições, pets) que a configuração não mostra. Faltam ainda o seletor de indicador, o IconStudio, a Licença (fora de escopo), a bandeja e a lista completa de widgets (só "Mídia e tempo" apareceu).

- **Referência detalhada** (2026-10-25): os dados observados (esquema das configurações, espaços de trabalho, os 31 tipos de widget, formato dos pacotes de ícones e catálogo da galeria, comportamentos lidos do registro de execução, processos e memória) ficam em [`docs/reference/primodock/`](../reference/primodock/README.md), sem código, ativos nem dados de licença.

## 2. Limites da réplica

| Replicar | Não replicar |
|---|---|
| Layout, proporções, temas, animações, opções e comportamento | Código descompilado, o executável, os arquivos de ícone do `IconPacks`, o catálogo da galeria |
| Nomes de opções e categorias de ícone como organização funcional | Marca, nome, logotipo, mascote (o gato) com o desenho original |
| Todas as variações de ícone (formas, pacotes, por item) | Licença/período de teste e o servidor de atualização (`dl.criador.studio`) |

Os ícones são redesenhados do zero, em SVG, com a mesma taxonomia (11 categorias). A licença do DockMax passou a ser de uso pessoal (ADR 0009), e a regra de não copiar arquivos continua. Ter visto o produto original não é um "clean room" estrito: a garantia é a implementação do zero, sem arquivos nem código do PrimoDock.

## 3. Inventário do PrimoDock

### 3.1 Dock e aparência
- **Temas (9, "como no RocketDock"):** Nativo, Liquid Glass, Meia-noite, Rocket (contorno azul com brilho), Prateleira Aqua (prateleira metálica inclinada), Neon (contorno ciano e magenta com brilho), Canarinho (amarelo-verde), Papel (branco) e Flutuante (sem barra, ícones soltos com reflexo). Os ids internos vistos na configuração são `floating`, `liquid` e `aqua`; há ainda parâmetros de tema sazonal (neve, capa de neve, luzes).
- **Personalizar o tema:** borda (liga/desliga), cantos (0 a 100 %, "o quanto a barra é arredondada"), reflexos (imagem espelhada embaixo de cada ícone) e "Restaurar" para voltar ao original. Na configuração há também cor superior e inferior, opacidade, cor e espessura da borda, brilho, cor do separador.
- **Posição:** Embaixo, No topo, Esquerda, Direita. Só o Dock muda de borda; a barra de tarefas e o Iniciar continuam onde estão.
- **Tamanho:** ícones (controle deslizante, 52 px) e espaço inferior (distância até a borda, 2 px).
- **Formato dos ícones:** Placas arredondadas (padrão), Mesmo tamanho, Como vieram.
- **Efeito ao passar o mouse:** Nenhum, Escala (com controle de 1,10×) e Ampliação (os vizinhos crescem, como no macOS).
- **Animação de abertura:** Pulo (padrão), Pulso, Tremor.
- **Indicador dos apps abertos:** estilos escolhidos num seletor (o padrão é um traço, `dash`) e cor do indicador: Padrão (branco) mais 15 cores, ou seguir a cor do workspace.
- **Claro ou escuro:** o Dock e as janelas seguem o modo do Windows (Automático).
- **Partes visíveis:** lançador, apps em execução, seletor de espaços de trabalho, status do sistema, relógio, data, ícones da bandeja.
- **Comportamento:** ocultar automaticamente (atraso de revelação), manter janelas fora da área do dock, um dock por monitor, prévias de janela (atraso; só com várias janelas), modo de economia de energia.

### 3.2 Integração com o Windows
- Registra uma **barra de aplicativos** (app bar) que reserva a área de trabalho, e oculta a barra de tarefas nativa (ocultar por completo ou ocultar automaticamente).
- **Hospeda a área de notificação:** painel com os ícones da bandeja e a opção de fixar alguns deles no dock.
- Lançador abre o menu Iniciar do Windows (opcional) e mantém um catálogo de apps instalados (168 de 197 entradas no teste).
- Lista janelas abertas, ativa/minimiza e mostra prévias.
- Vidro acrílico com cor de tinta, cantos arredondados, configurações rápidas.

### 3.3 Espaços de trabalho
- Vários espaços nomeados (Work, Personal, Focus) com cor própria; cada um tem sua lista de itens (apps, widgets, pilhas).
- Um espaço ativo por monitor; as janelas pertencem a um espaço e as dos demais ficam "estacionadas".
- Seletor com pontos de cor no dock.

### 3.4 Pacotes de ícones, Galeria e IconStudio
- **Ícones dos apps:** Original (os do Windows) ou um pacote (o instalado é Pixel System). Troca-se o pacote de todos os apps, ou o de um só pelo menu do item no Dock. Atalhos para a galeria, abrir a pasta de pacotes e recarregar pacotes.
- Cada pacote tem 11 categorias (sistema, arquivos, desenvolvimento, internet, produtividade, criativo, IA, segurança, jogos, utilidades, fallback), uma pasta `apps` e uma `_widget`; widgets seguem o pacote.
- **Galeria de ícones:** pacotes prontos, com um em destaque ("Primo", 198 ícones, uma família de cor por tipo de app) e variações (Sunset, Mono, Midnight; 207 ícones cada), "Cobre 7 dos seus 7 apps" e botão Instalar.
- **IconStudio:** editor de ícones por app (letras, imagem, ícone do app, pacotes). **Fora de escopo** por decisão do autor (2026-10-22): ele não usa. A aba não entra no DockMax.

### 3.5 Widgets
- **Catálogo:** "Coisas vivas dentro do Dock: um quadrado quando é um número, um cartão largo quando tem mais a dizer." Busca e categorias; clicar adiciona ao workspace. A primeira categoria, **Mídia e tempo**, tem: Tocando agora, Calendário, Calendário · Dia, Relógio, Analógico, World Clock, Timer de foco, Pomodoro e Contagem regressiva. As demais categorias, em ordem de rolagem:
  - **Sistema:** Bateria · Estado, Bateria · Anel de porcentagem, Bateria · Só o ícone, Atividade do sistema · Barras / Anéis / Números, Atividade de rede, Atividade de rede · Gráfico, Layout do teclado.
  - **Ferramentas:** Beber água, Capturas de tela, Arquivos.
  - **IA:** Consumo do Claude, Consumo do Claude · Cartão, Uso do Codex, Uso do Codex · Cartão, Uso do OpenCode, Uso do OpenCode · Provedores.
  - **Notas e clima:** Post-it, Lista de tarefas, Tarefas · Contagem, Clima · Agora, Clima · Previsão.
- **Variantes:** o mesmo dado aparece em variantes de forma (quadrado, cartão largo, anel, barras, gráfico); cada variante é uma entrada do catálogo.
- **Em uso:** lista dos widgets adicionados, cada um com miniatura, nome, lixeira e opções: Largura do tile (Um, Dois, Três ou Quatro ícones; só as larguras que o widget aceita), Estilo (Barras, Números, Anéis), chaves por campo (CPU, Memória) e **Mostrar nos workspaces** (um botão de cor por workspace, múltipla escolha).
- **Espaçadores:** "Espaços vazios entre os ícones do Dock", pequeno ou grande, por workspace; remove-se com o botão direito no espaçador.
- **Visual:** ladrilhos no estilo do pacote de ícones (no Pixel System, fonte pixelada e cantos arredondados), de 1 a 4 colunas.
- **Campos vistos na configuração:** atividade do sistema (CPU e memória), rede, relógio (digital/analógico, segundos, fuso), Pomodoro (foco, pausas, ciclos), temporizador com predefinições, texto, clima (cidade, coordenadas, °F), água (meta, copo), uso do Claude, Codex e OpenCode (escopo, orçamento de tokens), pasta, tarefas, música (arte do álbum).

### 3.6 Pet
"Um amiguinho que mora em cima do Dock. Corre quando o PC está ocupado e cochila quando está tranquilo." Espécies: Gato, Cachorro, Passarinho, Dragão, Dino, Guerreira, Guerreiro, Papai Noel e Fantasminha, em sprites de pixel art. Pelagem em 4 cores (laranja, cinza, preto, creme). Os sprites são desenhados do zero.

### 3.7 Estrutura das configurações e telas
- **Janela de configurações:** barra lateral com busca ("Buscar nos ajustes") e as abas **Workspaces, Widgets, Galeria de ícones, IconStudio, Aparência, Geral, Licença, Sobre**; selo "Teste grátis, faltam N dias" no rodapé (fora de escopo). Cada aba tem um título com ícone colorido em placa e uma frase de apoio.
- **Workspaces:** abas Trabalho/Pessoal/Foco com bolinha de cor, "+ Novo workspace"; cartão do workspace ativo com atalho **Ctrl+Alt+número**, contagem de apps e widgets, duplicar e excluir; prévia do Dock sobre um papel de parede com "arraste para reordenar, puxe para fora para remover, botão direito para renomear"; lista de apps (duplo clique adiciona ao fim, ou arrastar para a barra). Troca-se de workspace pelas bolinhas na barra, pela roda do mouse sobre elas ou com o atalho.
- **Geral:** barra de tarefas do Windows (Deixar visível, Ocultar automaticamente, Ocultar por completo, cada um com miniatura e explicação) e opções do Dock com chaves: Mostrar apps abertos (os não fixados, depois de um separador), Seletor de workspace no Dock (bolinhas coloridas no começo), Bateria, rede e volume (no fim do Dock com a barra oculta; abre as configurações rápidas do Windows) e outras.
- **Sobre:** ícone do app, versão, avisos de terceiros, texto "Um Dock para cada coisa que você faz. Workspaces, prévias ao vivo e um pet na barra. Sem conta, sem analytics, sem rastreamento." e botões Enviar feedback, Ver o tour, Site, Abrir pasta de dados.

### 3.7a Menus de contexto e prévia de janela
- **Menu do Dock** (botão direito no espaço vazio): Adicionar espaçador pequeno, Adicionar espaçador grande, Widgets…, Colocar comida (alimenta o pet), Efeito ao passar o mouse (submenu), Ocultar automaticamente, Mostrar apps abertos (com marca); seção **Workspaces** com os três e seus atalhos; Procurar atualizações…, Configurações…, Licença…, Encerrar.
- **Menu de um widget:** Largura do tile (submenu), Empilhar com (submenu; widgets empilhados), Editar widget…, Remover do Dock; depois a seção Workspaces e o rodapé do menu do Dock.
- **Prévia de janela:** ao passar o mouse num app aberto, um cartão sobre o ícone com o título da janela, um botão de fechar e a miniatura ao vivo.
- **Fim do Dock (barra nativa oculta):** ícones de bateria, rede e volume, ícones fixados da bandeja, a seta do painel da bandeja e relógio com data. O pet fica em pé sobre o Dock.
- **Visual do Dock com o pacote Pixel System:** bolinhas de workspace à esquerda, separadores verticais finos entre fixos, abertos e widgets, ladrilhos em placa com cantos arredondados e um traço branco sob o app aberto.

### 3.7b Lote 2 de capturas (2026-10-22)
- **Temas no Dock real:** Nativo = barra cinza translúcida, borda fina clara, cantos médios; Liquid Glass = pílula de vidro clara com borda branca e brilho interno; Meia-noite = barra escura com contorno branco fino e cantos pequenos; Rocket = barra azul-marinho com contorno e brilho azuis; Prateleira Aqua = sem barra, prateleira metálica prateada em perspectiva sob os ícones, com reflexos; Neon = barra preta com contorno degradê ciano-magenta e brilho; Canarinho = pílula amarelo-esverdeada com brilho; Papel = pílula branca opaca; Flutuante = sem barra, ícones soltos com reflexo. Todos usam o mesmo ladrilho (placa arredondada do pacote de ícones) e o pet em pé sobre a barra.
- **Temas sazonais:** seção "Sazonais — Temas de Halloween e de Natal" abaixo dos nove, com ao menos três (luzes laranja e morcegos, guirlanda verde com luzes, abóboras/luzes roxas). "Economia de bateria" para a animação deles.
- **Posições:** no topo, a barra fica centrada com a bandeja e o relógio à direita; à direita, o Dock vira coluna vertical com a bandeja e o relógio no topo, widgets com o texto girado, apps no meio e as bolinhas de workspace embaixo; à esquerda é o espelho.
- **Indicador:** dois estilos, **Ponto** (padrão) e **Traço**; a chave **Cor por workspace** (a cor de cada workspace marca os apps abertos) e a paleta de cor do indicador.
- **Ícones dos apps:** "Widgets seguem o pacote de ícones" (chave); Escala ao passar o mouse de 1,00× a 1,60× (visto 1,10×).
- **Menu de um app:** Nova janela, Remover do Dock, Mover para o workspace (submenu), Fechar janela, Encerrar, Mostrar no Explorador de Arquivos, Ícone (submenu), seção Workspaces, Procurar atualizações…, Configurações…, Licença…, Encerrar.
- **Prévia de janela:** cartão escuro acima do ícone com título da janela, botão fechar e miniatura ao vivo; o ícone do app fica ampliado durante a prévia.
- **Configurações rápidas:** clicar nos ícones de bateria, rede e volume abre o painel nativo do Windows (mídia, Wi-Fi, Bluetooth, brilho, volume), ancorado perto do canto.
- **Geral, em ordem:**
  - *Relógio:* mostrar a hora (clique abre o calendário do Windows) e **Mostrar a data**.
  - *Ícones de outros apps:* com a barra nativa oculta, uma seta no Dock abre o painel da bandeja; **Ícones da bandeja no Dock** (até 6): cada app da bandeja escolhe Painel, Dock ou Esconder.
  - *Botão Iniciar:* mostrar (primeiro no Dock, arrastável), "abre" Apps do PrimoDock ou o Iniciar do Windows, **Mostrar Downloads** (pasta em todo workspace), **Mostrar Lixeira**.
  - *Ocultação:* ocultar e mostrar automaticamente (o Dock aparece ao encostar na borda), **manter as janelas fora do Dock** (maximizadas param acima dele).
  - *Monitores:* **em todos os monitores** (cada um com seu Dock e seu workspace), e como o botão Iniciar se comporta nos outros monitores (o Windows só abre o Iniciar no principal; nos outros abre os apps do Dock no meio da tela).
  - *Prévias de janela:* prévia ao passar o mouse, atraso (0,2 s), só com 2 ou mais janelas.
  - *App:* **Economia de bateria** (sem pulo ao abrir, enfeites sazonais parados), Idioma (Sistema, Português, English).
  - *Inicialização:* abrir quando o Windows iniciar. *Atualizações:* versão, "Verificar agora".
  - *Atalhos:* Ctrl+Alt+1…9 vai ao workspace; a roda sobre o Dock troca de workspace; Ctrl+roda (ou roda lateral) troca o widget de uma pilha; parar o ponteiro sobre o ícone mostra as janelas; botão direito fixa, move ou fecha.

### 3.7c Requisito do autor além do PrimoDock: os menus nativos do Windows (2026-10-23)
O autor quer, no dock, **os mesmos menus que a barra de tarefas do Windows mostra** ao clicar com o botão direito num aplicativo: nos ícones do dock e nos da bandeja. O exemplo é o FortiClient, cujo menu da bandeja lista as VPNs salvas ("Connect to ..."), o que o PrimoDock não faz.

- **Bandeja:** o clique do botão direito vai ao próprio programa, que desenha o menu dele (a bandeja hospedada em `windows/tray_host.rs` manda as mensagens que a barra mandaria). O menu é, por construção, o mesmo que o do Windows, com o conteúdo que o programa montar na hora (as VPNs, por exemplo).
- **Ícones de aplicativos do dock (Fase B):** o menu da barra do Windows é a **Jump List** do aplicativo (tarefas, itens recentes e fixados) mais "Fixar/Desafixar" e "Fechar janela". O Windows não oferece API para mostrar o menu da barra, só para os aplicativos montarem a Jump List; as listas ficam em `%APPDATA%\Microsoft\Windows\Recent\AutomaticDestinations` e `CustomDestinations`. A Fase B precisa decidir entre ler esses arquivos (formato não documentado, mas estável) e usar o menu de contexto do atalho; fica registrada como requisito, com um spike antes de prometer.

### 3.8 Extras
Onboarding (tour), idioma do sistema e atualização automática.

## 4. Comparação com o DockMax v0.2.0

| Área | DockMax hoje | Distância |
|---|---|---|
| Dock | Barra flutuante com ampliação, 1 tema (claro/escuro), tamanho e opacidade | Faltam temas, personalização, formas de ícone, indicadores, animações |
| Barra de tarefas | Convive com a nativa (ADR 0004) | **Inversão**: passa a reservar a área e a ocultar a nativa |
| Itens | Só Configurações e Sobre; atalhos na Fase 3 | Falta catálogo de apps, fixar, reordenar, apps em execução |
| Espaços de trabalho | Inexistente | Novo modelo de dados e donos de janela |
| Ícones | Glifos próprios simples | Falta sistema de pacotes, por item, galeria e IconStudio |
| Configurações | Painel pequeno ancorado ao dock | Falta a janela com barra lateral, busca e as 8 abas |
| Widgets | Relógio, Progresso do tempo, Nota | Faltam 10 ou mais; o contrato de widget já comporta |
| Bandeja, prévias, multimonitor | Inexistentes | Maior risco técnico |
| Persistência | SQLite com migrações | Reaproveitável |

## 5. Decisões de arquitetura que esta spec abre

1. **Barra de aplicativos e ocultação da barra nativa** em Rust (`SHAppBarMessage`, crate `windows`). Reverte o princípio "convive com a barra" do `PRODUCT.md` e o ADR 0004: exige um ADR novo e a atualização do `PRODUCT.md`, `DESIGN.md` e roadmap.
2. **Bandeja:** o PrimoDock usa uma biblioteca .NET para hospedar a área de notificação. Em Rust não há equivalente pronto: ou se implementa um host de bandeja (alto risco), ou se começa só espelhando os ícones, ou se adia.
3. **Prévias de janela:** miniaturas do DWM precisam de uma janela nativa por cima do WebView2. Provavelmente fica como janela nativa auxiliar, não HTML.
4. **Memória:** o orçamento de ~220 MB do roadmap vale para o dock; vários monitores e prévias o pressionam. Mantém a medição em todo PR de interface.
5. **Janela única** (spike da Fase 4 do roadmap) ganha prioridade, porque o dock por monitor multiplica os renderizadores.

## 6. Fases propostas

Cada fase é um PR (ou uma série), com spec, plano e teste, como nas fases anteriores.

| Fase | Entrega | Risco |
|---|---|---|
| **A. Visual** | Sistema de temas (floating, liquid, aqua) com personalização, formas de ícone, hover, animação de abertura, indicador, posições; tokens novos | Baixo |
| **B. Itens e ícones** | Catálogo de apps (Menu Iniciar), fixar/reordenar, pacotes de ícones próprios com as 11 categorias, ícone por item e imagem personalizada | Médio |
| **C. Espaços de trabalho** | Modelo, seletor com pontos, indicador por espaço, itens por espaço | Médio |
| **D. Windows** | Barra de aplicativos, ocultar/auto-ocultar a barra nativa, um dock por monitor, apps em execução, ativar/minimizar, ocultar automático | Alto |
| **E. Prévias e bandeja** | Miniaturas das janelas; painel da bandeja e ícones fixados no dock | Alto |
| **F. Widgets** | Atividade, rede, clima, água, Pomodoro, tarefas, pasta, música, uso de Claude/Codex/OpenCode, texto | Médio por widget |
| **G. Extras** | Mascote, onboarding, configurações rápidas, idioma | Baixo |

Ordem sugerida: A → B → D → C → E → F → G. A fase D vem cedo porque é a que mais muda a arquitetura e o que mais pode inviabilizar o resto; um spike dela antes de A reduz o risco.

## 7. Pendências para o autor

- Capturas que ainda faltam: ver a lista pedida ao autor em 2026-10-22 (seletor de indicador, painel da bandeja, menu de um app, configurações dos widgets de relógio, clima, água e IA, e o Dock nos outros temas).
- Confirmar a ordem das fases e se o spike da barra de aplicativos vem primeiro.
- Feito em 2026-10-22: `PRODUCT.md`, ADR 0009 (substitui a convivência do ADR 0004) e licença de uso pessoal. Falta atualizar o roadmap quando a ordem das fases for aprovada.
