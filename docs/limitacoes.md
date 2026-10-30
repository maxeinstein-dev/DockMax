# Limitações conhecidas

O que o Dockyard (antes DockMax) **não** faz hoje, e o que pode surpreender. Vale para a versão 0.7.0. Nada aqui é um defeito escondido: cada item é uma decisão ou um corte de escopo. O andamento está no [roadmap](roadmap.md).

## Sistema

- **Só Windows 11**, testado com um e com dois monitores, com escala de 100 % e de 125 %. Windows 10 não foi testado.
- **O dock some quando o monitor dele está em tela cheia** (um vídeo, uma apresentação) e volta quando ela acaba, mesmo que você use o outro monitor nesse meio tempo ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md)).
- **Um dock em cada monitor**, cada um reservando a borda do seu ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md), D4b). Todos mostram os mesmos itens; não há itens diferentes por monitor. Em Configurações › "Dock em todos os monitores" dá para deixar só o do monitor principal: com a barra do Windows oculta, os outros monitores ficam sem barra nenhuma.
- **Espaços de trabalho** ([spec 0013](specs/0013-fase-c-espacos-de-trabalho.md)): cada espaço guarda as suas janelas, como as áreas de trabalho do Windows, mas por monitor: trocar de espaço num monitor esconde as janelas dele que são de outros espaços (elas saem também do Alt+Tab) e traz de volta as do espaço novo. O Dockyard não usa as áreas de trabalho do Windows: o Win+Tab continua mostrando as dele, e as duas coisas convivem. Ao fechar o Dockyard (ou se ele cair), todas as janelas escondidas voltam; ao abrir de novo, cada uma volta para o seu espaço. Com dois ou mais espaços, `Ctrl+Alt+1…9` troca de espaço, e no teclado ABNT2 isso toma o AltGr+1, 2 e 3 (¹ ² ³) enquanto houver espaços nessas posições. "Mover para <espaço>", no menu de um programa, leva só as janelas dele no espaço em uso; uma janela que acabou de abrir (antes de o Dockyard olhar as janelas) ainda não vai junto.
- **Os painéis do Windows abrem no monitor principal.** Rede, volume e bateria abrem as configurações rápidas (Win+A), e o relógio abre o calendário (Win+N), mesmo clicados no dock de outro monitor. O Windows só os abre por esses atalhos e escolhe sozinho onde.
- **O instalador não é assinado.** O SmartScreen do Windows avisa na instalação e é preciso escolher "Mais informações › Executar assim mesmo". Para compilar do código-fonte, o Controle Inteligente de Aplicativos precisa estar desativado (README).
- **Só inicia com o Windows se você ligar "Iniciar com o Windows" em Configurações** (desligado por padrão).
- **Leitor de tela nunca foi testado.** Os controles têm nomes e papéis acessíveis e há testes de acessibilidade por código, mas ninguém usou o aplicativo com um leitor de tela. O dock, desenhado sem navegador ([spec 0011](specs/0011-arquitetura-hibrida.md)), se apresenta à UI Automation como uma barra de ferramentas com um botão por item, que o leitor de tela pode acionar.
- **O dock não recebe o teclado.** Ele nunca é ativado (o programa em uso continua na frente); os painéis e o app de Configurações recebem.
- **Alt+Espaço abre o lançador** (spec 0012), não o menu de sistema da janela em foco (Restaurar, Mover, Fechar), que continua no clique direito da barra de título. Em Configurações › Geral dá para trocar o atalho por outra combinação ou desligar a chave "Lançador" e devolver o Alt+Espaço ao Windows. O atalho aceita só letras, números, F1–F24 e Espaço, com Ctrl, Alt, Shift ou Win; Win sozinho com uma tecla não é aceito (o Windows usa quase todas). Se outro programa já tiver o atalho (o PowerToys Run, por exemplo), o Dockyard avisa ali e o lançador abre por "Buscar apps…", no menu do dock ou da bandeja.
- **O lançador acha** os apps da pasta Aplicativos (os do menu Iniciar), **contas**, **conversões de unidades e moedas**, **buscas na web por palavra-chave**, uma **lista fixa de páginas de configurações** (34 do Windows e as seções do Dockyard), **arquivos e pastas**, **caminhos** digitados (`C:\`, `~\`, `%APPDATA%\`, compartilhamentos de rede), as **janelas abertas** e o **histórico da área de transferência** do Windows (Win+V). Uma página do Windows fora da lista não aparece. Arquivos e pastas vêm do Everything, quando ele está rodando, e do índice do Windows Search: o que nenhum dos dois indexa não aparece. A área de transferência só mostra algo com o histórico do Windows ligado. Extensões ficam para depois ([spec 0012](specs/0012-lancador.md)).
- **A busca na web abre só endereços `https://`**: uma palavra-chave com `http://` ou outro esquema não é aceita em Configurações.
- **Idioma:** português e inglês (o seletor Idioma, em Configurações; "Sistema" segue o idioma de exibição do Windows: português se for um, inglês nos outros). Não há outros idiomas.

## A barra de tarefas

- O dock **esconde** a barra do Windows e a **devolve** como estava ao encerrar, de qualquer jeito que o programa termine (um vigia separado cuida disso). Se o computador for desligado à força com o Dockyard aberto, o estado fica gravado e é restaurado na próxima vez que o Dockyard abrir; se ele não voltar a abrir, a barra pode continuar oculta: reabra o Dockyard e feche-o, ou mude "Barra de tarefas do Windows" para "Visível" em Configurações.
- **O dock sabe quais programas fixados estão abertos** ([spec 0009](specs/0009-fase-d-janelas.md), D1): eles ganham o indicador, e o clique traz a janela para a frente, minimiza se você já está nela ou passa para a próxima janela do programa. Os programas abertos que **não** estão fixados aparecem depois dos widgets (D2; dá para esconder em Configurações › Mostrar apps abertos), com o mesmo clique (o botão do meio abre uma nova instância, ou nada, em Configurações), e o botão direito abre um menu (nova instância, fechar janela, fixar no dock; nos fixados, "Fechar janela" junto da Jump List). Fechar pede ao programa que feche cada janela, como o botão de fechar: se ele perguntar se quer salvar, a pergunta aparece. Passar o mouse sobre um programa aberto mostra o cartão com as miniaturas das janelas ([spec 0015](specs/0015-fase-e-previas.md)).
- Um programa **encerrado à força** (Gerenciador de Tarefas, falha) perde o indicador em até uns 5 segundos, não na hora: o Windows não avisa que as janelas dele sumiram. Fechar a janela normalmente apaga o indicador na hora.
- Para ativar uma janela o Windows exige que o programa tenha recebido a última entrada do usuário; o Dockyard usa o recurso de compartilhar a entrada com a janela da frente quando o pedido simples é recusado. Se o Windows ainda recusar, a janela só pisca na barra de tarefas (que está oculta): nesse caso o clique não a traz.
- A bandeja do sistema é hospedada pelo Dockyard (painel da seta), mas só mostra o que o programa registrou na área de notificação; ícones ocultos pelo Windows aparecem como o Windows os informa.

## Apps fixados

- No máximo **40 programas** fixados.
- O **menu do programa** (botão direito) usa a Jump List do Windows: depende do que o próprio programa grava. Programas que não gravam nada mostram só "Abrir" e "Remover do dock". O formato dos arquivos da Jump List não é documentado pela Microsoft; ele foi lido por engenharia reversa e pode mudar numa atualização do Windows (o menu então fica sem as tarefas, sem derrubar o dock).
- **Soltar no dock** vale para `.exe` e `.lnk` em disco local (`C:\...`): pastas, outros arquivos e caminhos de rede (`\\servidor\...`) são ignorados.
- **Não há troca de ícone por programa nem imagem própria.** O ícone vem do Windows ou do **pacote de ícones** escolhido em Aparência › Ícones dos apps, que vale para todos os programas de uma vez. Vêm 15 pacotes embutidos (Retrô 8-bit, Esmalte, Traço, Clássico 98, Círculo, Contorno colorido, Tom sobre tom, Vidro, Neon, Argila, Fósforo, Game Boy, Tático, Medieval e Caçador; [spec 0019](specs/0019-fase-g-extras.md) e [spec 0020](specs/0020-temas.md)), cada um desenhado para uma lista fixa de programas comuns; um programa fora da lista fica com a imagem de reserva do pacote. A escolha de um ícone por programa ficou para o futuro.

## Widgets e dados

- Os widgets são **Relógio, Progresso do tempo, Nota, Atividade, Atividade de rede, Calendário, Ciclo, Contagem regressiva, Pomodoro, Lista de tarefas, Beber água, Tocando agora, Uso do Claude, Clima e Lançador** ([spec 0017](specs/0017-fase-f-widgets.md)). Atalho e Arquivos ainda não existem como widget, e não há pilhas de widgets.
- **Calendário e Ciclo pedem uma conta Google** (Configurações › Contas). Não há iCal nem outras agendas.
- A **Nota** é texto simples, sem formatação.
- Os dados ficam **só nesta máquina** (`%APPDATA%\dev.maxsueleinstein.dockmax`; a pasta e o banco continuam com o nome antigo de propósito, sem migração): não há nuvem nem sincronização. Para mudar de computador, copie essa pasta com o Dockyard fechado. A conta Google fica no cofre de credenciais do Windows e precisa ser conectada de novo no outro computador.
- **Os arquivos de segurança nunca são apagados pelo programa.** Cada migração do banco deixa um `dockmax.db.bak-v<n>` (pequeno) e um banco danificado é renomeado para `dockmax.db.corrompido-<data>`. Apagar esses arquivos é decisão de quem usa.

## O que usa a rede

O Dockyard não tem servidor e não manda dados de uso a ninguém. Só estas partes falam com a internet, e só quando usadas:

- **Atualização automática:** procura uma versão nova nos Releases do GitHub do repositório 15 s depois de abrir e a cada 6 horas (só no build de release). Com uma versão nova, avisa uma vez; baixa e instala só depois do clique em **Atualizar** ([spec 0019](specs/0019-fase-g-extras.md), G3).
- **Conta Google** (Calendário, Ciclo e tarefas): fala com as APIs do Google; o que é lido e onde fica está na [política de privacidade](privacidade.md).
- **Clima:** manda ao Open-Meteo o nome da cidade digitada (para achar as coordenadas) e as coordenadas, a cada 30 minutos enquanto um dock tem o widget. Não há localização automática.
- **Uso do Claude:** pergunta à Anthropic o uso da conta do Claude Code, com o acesso que o próprio Claude Code guarda no computador, a cada 5 minutos enquanto um dock tem o widget. O acesso é lido a cada consulta e nunca é copiado nem vai para os registros.
- **Moedas no lançador:** as cotações do Banco Central Europeu vêm do Frankfurter, no máximo uma vez por dia, ao abrir o lançador; sem rede e sem cotação guardada, a conversão de moedas não aparece.
- **Busca na web do lançador:** só abre o navegador no endereço da palavra-chave; o Dockyard não faz a busca.

## Desempenho

- O dock é desenhado em Rust nativo, **sem WebView2** ([spec 0011](specs/0011-arquitetura-hibrida.md)): em repouso é um processo só, com cerca de 65 MB de memória privada (medido na 0.3.0; de lá para cá entrou muita coisa e o repouso não foi medido de novo). Os painéis e o app de Configurações são WebView2, criados quando abrem: o **primeiro painel** depois de um tempo parado leva cerca de 0,4 s para aparecer (passar o mouse sobre o item antes do clique adianta a criação). A janela do painel é liberada 45 s depois de fechado, e o WebView2 sai com ela.
- Se o processo do WebView2 cair com um painel aberto (por exemplo, uma falha do driver de vídeo), o Dockyard se reinicia sozinho uma vez; se cair de novo em menos de 2 minutos, ele fecha e a barra do Windows volta ([ADR 0010](adr/0010-gpu-no-processo-do-navegador.md)).
