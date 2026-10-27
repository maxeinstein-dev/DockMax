# Limitações conhecidas

O que o DockMax **não** faz hoje, e o que pode surpreender. Vale para o MVP (ver "Escopo do MVP" no [roadmap](roadmap.md)). Nada aqui é um defeito escondido: cada item é uma decisão ou um corte de escopo.

## Sistema

- **Só Windows 11**, testado com um e com dois monitores, com escala de 100 % e de 125 %. Windows 10 não foi testado.
- **O dock some quando o monitor dele está em tela cheia** (um vídeo, uma apresentação) e volta quando ela acaba, mesmo que você use o outro monitor nesse meio tempo ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md)).
- **Um dock em cada monitor**, cada um reservando a borda do seu ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md), D4b). Todos mostram os mesmos itens; não há itens diferentes por monitor. Em Configurações › "Dock em todos os monitores" dá para deixar só o do monitor principal: com a barra do Windows oculta, os outros monitores ficam sem barra nenhuma.
- **O instalador não é assinado.** O SmartScreen do Windows avisa na instalação e é preciso escolher "Mais informações › Executar assim mesmo". Para compilar do código-fonte, o Controle Inteligente de Aplicativos precisa estar desativado (README).
- **Só inicia com o Windows se você ligar "Iniciar com o Windows" em Configurações** (desligado por padrão).
- **Leitor de tela nunca foi testado.** Os controles têm nomes e papéis acessíveis e há testes de acessibilidade por código, mas ninguém usou o aplicativo com um leitor de tela. O dock, desenhado sem navegador ([spec 0011](specs/0011-arquitetura-hibrida.md)), se apresenta à UI Automation como uma barra de ferramentas com um botão por item, que o leitor de tela pode acionar.
- **O dock não recebe o teclado.** Ele nunca é ativado (o programa em uso continua na frente); os painéis e o app de Configurações recebem.
- **Alt+Espaço abre o lançador** (spec 0012), não o menu de sistema da janela em foco (Restaurar, Mover, Fechar), que continua no clique direito da barra de título. Em Configurações › Geral dá para desligar a chave "Lançador" e devolver o Alt+Espaço ao Windows. Se outro programa já tiver o atalho (o PowerToys Run, por exemplo), o DockMax avisa ali e o lançador abre por "Buscar apps…", no menu do dock ou da bandeja.
- **O lançador acha os apps** da pasta Aplicativos (os do menu Iniciar), **contas**, **buscas na web por palavra-chave** e uma **lista fixa de páginas de configurações** (34 do Windows e as seções do DockMax): uma página do Windows fora da lista não aparece. Arquivos, janelas abertas e extensões ficam para depois ([spec 0012](specs/0012-lancador.md)).
- **A busca na web abre só endereços `https://`**: uma palavra-chave com `http://` ou outro esquema não é aceita em Configurações.

## A barra de tarefas

- O dock **esconde** a barra do Windows e a **devolve** como estava ao encerrar, de qualquer jeito que o programa termine (um vigia separado cuida disso). Se o computador for desligado à força com o DockMax aberto, o estado fica gravado e é restaurado na próxima vez que o DockMax abrir; se ele não voltar a abrir, a barra pode continuar oculta: reabra o DockMax e feche-o, ou mude "Barra de tarefas do Windows" para "Visível" em Configurações.
- **O dock sabe quais programas fixados estão abertos** ([spec 0009](specs/0009-fase-d-janelas.md), D1): eles ganham o indicador, e o clique traz a janela para a frente, minimiza se você já está nela ou passa para a próxima janela do programa. Os programas abertos que **não** estão fixados aparecem depois dos widgets (D2; dá para esconder em Configurações › Mostrar apps abertos), com o mesmo clique (o botão do meio abre uma nova instância, ou nada, em Configurações), e o botão direito abre um menu (nova instância, fechar janela, fixar no dock; nos fixados, "Fechar janela" junto da Jump List). Fechar pede ao programa que feche cada janela, como o botão de fechar: se ele perguntar se quer salvar, a pergunta aparece. Também não há prévia de janela (fase E).
- Um programa **encerrado à força** (Gerenciador de Tarefas, falha) perde o indicador em até uns 5 segundos, não na hora: o Windows não avisa que as janelas dele sumiram. Fechar a janela normalmente apaga o indicador na hora.
- Para ativar uma janela o Windows exige que o programa tenha recebido a última entrada do usuário; o DockMax usa o recurso de compartilhar a entrada com a janela da frente quando o pedido simples é recusado. Se o Windows ainda recusar, a janela só pisca na barra de tarefas (que está oculta): nesse caso o clique não a traz.
- A bandeja do sistema é hospedada pelo DockMax (painel da seta), mas só mostra o que o programa registrou na área de notificação; ícones ocultos pelo Windows aparecem como o Windows os informa.

## Apps fixados

- No máximo **40 programas** fixados.
- O **menu do programa** (botão direito) usa a Jump List do Windows: depende do que o próprio programa grava. Programas que não gravam nada mostram só "Abrir" e "Remover do dock". O formato dos arquivos da Jump List não é documentado pela Microsoft; ele foi lido por engenharia reversa e pode mudar numa atualização do Windows (o menu então fica sem as tarefas, sem derrubar o dock).
- **Soltar no dock** vale para `.exe` e `.lnk` em disco local (`C:\...`): pastas, outros arquivos e caminhos de rede (`\\servidor\...`) são ignorados.
- O ícone vem do Windows: **não há troca de ícone por programa nem imagem própria**. A leitura de pacotes de ícones existe (spec 0007, B4a) mas não há nenhum pacote embutido, e a escolha de um ícone por programa ficou para o futuro.

## Widgets e dados

- Os widgets são **Relógio, Progresso do tempo e Nota**. Os demais do PrimoDock (clima, rede, atividade, tarefas…) não existem.
- A **Nota** é texto simples, sem formatação.
- Os dados ficam **só nesta máquina** (`%APPDATA%\dev.maxsueleinstein.dockmax`): não há conta, nuvem nem sincronização. Para mudar de computador, copie essa pasta com o DockMax fechado.
- **Os arquivos de segurança nunca são apagados pelo programa.** Cada migração do banco deixa um `dockmax.db.bak-v<n>` (pequeno) e um banco danificado é renomeado para `dockmax.db.corrompido-<data>`. Apagar esses arquivos é decisão de quem usa.

## Desempenho

- O dock é desenhado em Rust nativo, **sem WebView2** ([spec 0011](specs/0011-arquitetura-hibrida.md)): em repouso é um processo só, com cerca de 65 MB de memória privada. Os painéis e o app de Configurações são WebView2, criados quando abrem: o **primeiro painel** depois de um tempo parado leva cerca de 0,4 s para aparecer (passar o mouse sobre o item antes do clique adianta a criação). A janela do painel é liberada 45 s depois de fechado, e o WebView2 sai com ela.
- Se o processo do WebView2 cair com um painel aberto (por exemplo, uma falha do driver de vídeo), o DockMax se reinicia sozinho uma vez; se cair de novo em menos de 2 minutos, ele fecha e a barra do Windows volta ([ADR 0010](adr/0010-gpu-no-processo-do-navegador.md)).
