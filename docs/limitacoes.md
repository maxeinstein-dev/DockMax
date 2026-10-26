# Limitações conhecidas

O que o DockMax **não** faz hoje, e o que pode surpreender. Vale para o MVP (ver "Escopo do MVP" no [roadmap](roadmap.md)). Nada aqui é um defeito escondido: cada item é uma decisão ou um corte de escopo.

## Sistema

- **Só Windows 11**, testado em um monitor principal com escala de 100 % e de 125 %. Windows 10 não foi testado.
- **O dock some quando o monitor dele está em tela cheia** (um vídeo, uma apresentação) e volta quando ela acaba, mesmo que você use o outro monitor nesse meio tempo ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md)).
- **Só o monitor principal recebe o dock.** Com a barra do Windows oculta, a do segundo monitor também some e esse monitor fica sem barra nenhuma ([ADR 0009](adr/0009-dock-substitui-a-barra-de-tarefas.md)). Um dock por monitor é da fase D.
- **O instalador não é assinado.** O SmartScreen do Windows avisa na instalação e é preciso escolher "Mais informações › Executar assim mesmo". Para compilar do código-fonte, o Controle Inteligente de Aplicativos precisa estar desativado (README).
- **Só inicia com o Windows se você ligar "Iniciar com o Windows" em Configurações** (desligado por padrão).
- **Leitor de tela nunca foi testado.** Os controles têm nomes e papéis acessíveis e há testes de acessibilidade por código, mas ninguém usou o aplicativo com um leitor de tela.

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

- Se o processo do WebView2 cair (por exemplo, uma falha do driver de vídeo), o DockMax se reinicia sozinho uma vez; se cair de novo em menos de 2 minutos, ele fecha e a barra do Windows volta ([ADR 0010](adr/0010-gpu-no-processo-do-navegador.md)).
- Cada janela do dock é um WebView2. A memória privada em repouso (processo e WebView2) tem como meta cerca de 220 MB; a medição mais recente está na seção "Medição de recursos" do [roadmap](roadmap.md).
