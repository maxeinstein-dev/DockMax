# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

(Aplicativo desktop para Windows 11 com interface web: Tauri 2 + React, renderizada pelo WebView2. A linguagem visual é web, desenhada para conviver com o shell do Windows; não é iOS nem Android.)

## Users

- **Primário:** o próprio autor, desenvolvedor, no Windows 11 do dia a dia (monitor 2560×1080 a 100%, efeitos de transparência desligados). Usa o DockMax o dia inteiro como o dock do Windows: abrir apps, trocar de workspace, ver widgets. A barra de tarefas nativa fica visível, escondida automaticamente ou oculta por completo, à escolha dele.
- **Secundário:** quem lê o projeto no GitHub (README, prints). A licença é de uso pessoal; o código pode ser estudado, não redistribuído.
- **Prioridade confirmada:** uso pessoal. O dock precisa ser agradável no uso diário.

## Product Purpose

Um dock completo para Windows que pode substituir a barra de tarefas: atalhos e apps abertos, workspaces com Docks próprios, widgets vivos, pacotes de ícones e temas prontos. Desde 2026-10-22 o produto mira a paridade de funcionalidades e de visual com o PrimoDock (spec 0004), implementada do zero. Sucesso na Fase 1: dock e painéis abrem e fecham sem falhas, preferências (tema, posição, tamanho, opacidade) sobrevivem a reinicializações, e o dock nunca atrapalha o trabalho nos apps de trás.

## Positioning

Dock para Windows construído com Tauri (leve, local, sem conta, sem analytics), com a mesma proposta de produto do PrimoDock: um Dock para cada coisa que se faz (workspaces), prévias de janela ao vivo e um pet na barra. Captura o mouse só onde desenha e deixa o clique atravessar o resto.

## Operating Context

- Fica sempre no topo, centralizado numa das quatro bordas (embaixo, no topo, esquerda, direita), com espaço configurável até a borda. Sobre o destino final, a barra de aplicativos reserva a área de trabalho (ADR 0009).
- Abre painéis ancorados aos itens; um painel por vez; fecha com Esc, clique fora ou novo clique no item.
- Controlado também pela bandeja do Windows (mostrar/ocultar, sair) enquanto a barra nativa estiver visível; com ela oculta, o próprio Dock hospeda a bandeja.
- Uso contínuo ao longo do dia, ao lado de navegadores, IDEs e chats; ocupa uma faixa pequena da tela.

## Capabilities and Constraints

- **Fase 1:** itens Configurações e Sobre; temas Sistema/Claro/Escuro; posição Embaixo/Em cima; tamanho por presets (Pequeno/Médio/Grande) sobre um número; opacidade Translúcido/Sólido.
- **Técnico (provado no PR 1, ver `docs/adr/0006-janelas-do-dock-e-paineis.md`):**
  - janelas separadas para dock (barra de 64 px lógicos + folga transparente de 40 px para ampliação) e painel;
  - ampliação viável, com captura do mouse só na barra e no ícone ampliado;
  - **sem desfoque real do fundo**: o vidro é simulado em CSS (camada tingida translúcida), porque o backdrop do Windows não respeita cantos arredondados;
  - CSP estrita em release: sem estilos inline injetados, sem `eval`.
- **Paridade com o PrimoDock** (spec 0004): 9 temas, formatos de ícone, efeitos de passar o mouse, animações de abertura, indicadores, workspaces, galeria de ícones, widgets em quadrados e cartões, pet, controle da barra de tarefas nativa. A ordem e o escopo ficam na spec.
- Interface em PT-BR, estruturada para receber inglês depois.

## Brand Commitments

- Nome: **DockMax**. Projeto independente, sem afiliação com o PrimoDock; a marca, o nome e o logotipo do PrimoDock não são usados.
- **Licença:** uso pessoal, todos os direitos reservados (arquivo `LICENSE`). Versões até a 0.2.0 foram MIT.
- Identidade **própria do DockMax**, sem ligação com a marca pessoal do autor, isolada em tokens para poder mudar depois.
- **Personalização é requisito:** o dock precisa ser ajustável o suficiente para ficar do gosto de quem usa. Toda a identidade vive em tokens trocáveis; os ajustes da Fase 1 são tema, posição, tamanho e opacidade, e a arquitetura não pode impedir outros (destaque, estilo dos ícones etc.).
- **Destino:** substituir a barra de tarefas do Windows, em harmonia com as janelas abertas (ADR 0009, que substitui a convivência do ADR 0004). Enquanto isso não chega, o dock convive com a barra.
- Ficar próximo do PrimoDock e do dock do macOS é desejado. Os temas chamativos (Neon, Rocket, Canarinho) existem como opção; o padrão continua discreto.
- Referência visual principal: o PrimoDock, observado pelas telas de configuração e pelo uso (capturas do autor). Referências secundárias: dock do macOS (Big Sur), WhiteSur, Zorin OS, Plank, Latte Dock, Cairo-Dock.
- **Regra de implementação do zero:** o DockMax copia funcionalidade, organização e aparência, nunca arquivos. Nada do executável do PrimoDock, do código descompilado, da pasta `IconPacks`, do catálogo da galeria nem dos sprites do pet entra no repositório. Ícones, pacotes e pet são desenhados do zero, e pacotes de terceiros (GPL, por exemplo) também ficam de fora. Ter visto o produto original não é um "clean room" estrito; a proteção é a regra de não copiar arquivos nem código.

## Evidence on Hand

- Documentação: `README.md`, `docs/architecture.md`, `docs/roadmap.md`, ADRs 0001–0009, specs 0001–0004.
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
