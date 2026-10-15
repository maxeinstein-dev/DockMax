---
version: 1
slug: "docs-design-dock-mock-html"
primary_target: "docs/design/dock-mock.html"
related_targets: []
---

# Surface brief — mock do dock (Fase 1)

## Escopo e modo

Mock de decisão em HTML estático (`docs/design/dock-mock.html`) para o usuário escolher entre as variantes B1 e B2 da direção B, antes do PR 2. Modo: Operate. O mock simula o dock real sobre um desktop do Windows 11 (papel de parede, janela aberta, barra de tarefas), com os mesmos controles do painel de Configurações da Fase 1 funcionando.

## Público, tarefa e prova

O autor, que usa o dock o dia inteiro, escolhe a variante e os tokens que o PR 2 vai implementar. Prova: o dock no seu tamanho real (ícones de 40/48/64 px), nos dois temas, nas duas opacidades, nas duas bordas, com hover, foco, ativo e painel aberto. Itens além de Configurações e Sobre são ilustrativos e rotulados como tal.

## Restrições

- Sem desfoque real do fundo (ADR 0006): vidro simulado.
- O app tem CSP estrita: os tokens têm de virar variáveis CSS em arquivos estáticos; nada de estilo injetado em runtime.
- Fonte do sistema (Segoe UI Variable) no app; o mock não carrega fontes externas.
- Nada chamativo no uso diário; personalização por tokens.

## Direction contract

THESIS: O dock do macOS que o autor gosta, refeito como peça do Windows 11: silhueta e ampliação de Big Sur, materiais e cantos que conversam com as janelas do Windows. Recusa a barra escura genérica de launcher e o arco-íris de ícones da Apple.

OWN-WORLD: Vidro simulado claro ("porcelana fosca") ou escuro ("grafite fosco"), borda interna de 1 px que pega luz no topo, sombra macia deslocada para baixo. Ícones em quadrado arredondado de cerâmica esmaltada: paleta própria de tons terrosos e frios dessaturados (ardósia, sálvia, ocre, argila, ameixa, tinta), gradiente de dois tons da mesma matiz, glifo de traço branco 1,75. Destaque único azul-cobalto para foco, seleção e indicador ativo.

STORY: O usuário vê o dock no desktop real, alterna B1/B2, tema, opacidade, tamanho e borda, abre os painéis e escolhe uma variante sabendo exatamente como ela fica às 18h com o Chrome atrás.

FIRST VIEWPORT: À esquerda, uma barra estreita de controles do mock (variante, papel de parede, estados). Ocupando o resto, o palco: desktop do Windows em escala 1:1 com uma janela aberta, a barra de tarefas no rodapé e o dock 8 px acima dela, com o painel de Configurações aberto sobre o item ativo. Abaixo do palco, a folha de estados e a tabela de tokens da variante ativa.

FORM: Direção fixada pelo usuário (B, variantes B1 Big Sur e B2 Prateleira), sem sorteio de conceitos. Assinatura: B1, ampliação em degraus com rótulo em pílula; B2, ampliação parabólica contínua sobre prateleira com reflexo curto. Seed key: none (pinned).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
