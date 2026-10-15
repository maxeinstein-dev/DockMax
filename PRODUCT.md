# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Aplicativo desktop para Windows 11 com interface web: Tauri 2 + React, renderizada pelo WebView2. A linguagem visual é web, desenhada para conviver com o shell do Windows; não é iOS nem Android.)

## Users

- **Primário:** o próprio autor, desenvolvedor, no Windows 11 do dia a dia (monitor 2560×1080 a 100%, efeitos de transparência desligados). Usa o DockMax o dia inteiro, junto da barra de tarefas nativa, para abrir painéis e, nas próximas fases, atalhos e widgets.
- **Secundário:** quem avalia o projeto como portfólio no GitHub (README, prints, demonstração).
- **Prioridade confirmada:** uso pessoal primeiro. O dock precisa ser agradável e discreto no uso diário; o valor de portfólio é consequência de um produto bem feito.

## Product Purpose

Um dock personalizável para Windows, inspirado em docks de macOS e Linux, que reúne atalhos e painéis de widgets sem substituir a barra de tarefas. Sucesso na Fase 1: dock e painéis abrem e fecham sem falhas, preferências (tema, posição, tamanho, opacidade) sobrevivem a reinicializações, e o dock nunca atrapalha o trabalho nos apps de trás.

## Positioning

Dock com identidade própria para Windows, construído com Tauri (leve, local, sem backend), que convive com a barra nativa em vez de substituí-la, captura o mouse só onde desenha e deixa o clique atravessar o resto.

## Operating Context

- Fica sempre no topo, centralizado na borda inferior (ou superior) do monitor principal, 8 px acima da área útil.
- Abre painéis ancorados aos itens; um painel por vez; fecha com Esc, clique fora ou novo clique no item.
- Controlado também pela bandeja do Windows (mostrar/ocultar, sair).
- Uso contínuo ao longo do dia, ao lado de navegadores, IDEs e chats; ocupa uma faixa pequena da tela.

## Capabilities and Constraints

- **Fase 1:** itens Configurações e Sobre; temas Sistema/Claro/Escuro; posição Embaixo/Em cima; tamanho por presets (Pequeno/Médio/Grande) sobre um número; opacidade Translúcido/Sólido.
- **Técnico (provado no PR 1, ver `docs/adr/0006-janelas-do-dock-e-paineis.md`):**
  - janelas separadas para dock (barra de 64 px lógicos + folga transparente de 40 px para ampliação) e painel;
  - ampliação viável, com captura do mouse só na barra e no ícone ampliado;
  - **sem desfoque real do fundo**: o vidro é simulado em CSS (camada tingida translúcida), porque o backdrop do Windows não respeita cantos arredondados;
  - CSP estrita em release: sem estilos inline injetados, sem `eval`.
- **Próximas fases:** atalhos de apps, arquivos e prateleira (Fase 3); widgets como relógio, notas, calendário etc. (Fase 2 em diante); múltiplos monitores e tela cheia (Fase 4).
- Interface em PT-BR, estruturada para receber inglês depois.

## Brand Commitments

- Nome: **DockMax**. Projeto independente, sem afiliação com o PrimoDock.
- Identidade **própria do DockMax**, sem ligação com a marca pessoal do autor, isolada em tokens para poder mudar depois.
- **Personalização é requisito:** o dock precisa ser ajustável o suficiente para ficar do gosto de quem usa. Toda a identidade vive em tokens trocáveis; os ajustes da Fase 1 são tema, posição, tamanho e opacidade, e a arquitetura não pode impedir outros (destaque, estilo dos ícones etc.).
- **Destino de longo prazo:** substituir a barra de tarefas do Windows, em harmonia com as janelas abertas (ADR 0004). O visual tem de conviver bem com janelas do Windows 11 desde já.
- Ficar próximo do dock do macOS é desejado, não um risco. O que deve ser evitado é ser chamativo no uso diário (brilho, cor ou animação que cansam).
- Referências visuais escolhidas pelo usuário: dock do macOS (Big Sur), tema **WhiteSur** para GTK, dock do **Zorin OS**; inspiração em docks Linux (Plank, Latte Dock, Cairo-Dock). Só como referência: nenhum código, ícone ou arquivo de tema de terceiros (várias são GPL; o DockMax é MIT).

## Evidence on Hand

- Documentação: `README.md`, `docs/architecture.md`, `docs/roadmap.md`, ADRs 0001–0006, specs 0001–0002.
- App funcional com visual provisório (barra escura com dois botões), CI verde.
- Não existem logotipo, ícones próprios, prints de produto nem demonstração; não inventar.

## Product Principles

1. **Nunca atrapalhar:** o dock não rouba foco, não bloqueia cliques fora do que desenha e não aparece no Alt+Tab.
2. **Discreto no uso diário, caprichado no detalhe:** a identidade aparece no acabamento, não no volume.
3. **Funciona em qualquer configuração do Windows:** sem depender de efeitos de transparência; legível em qualquer papel de parede, nos dois temas e nas duas opacidades.
4. **Identidade trocável:** toda decisão visual vive em tokens.
5. **Honestidade de dados:** o que estiver indisponível aparece como indisponível, nunca inventado.

## Accessibility & Inclusion

- Itens navegáveis por teclado (Tab e setas) com foco visível; `aria-label` em todo item só com ícone.
- Contraste AA nos dois temas e nas duas opacidades.
- Animações desligadas com "reduzir movimento" do Windows.
