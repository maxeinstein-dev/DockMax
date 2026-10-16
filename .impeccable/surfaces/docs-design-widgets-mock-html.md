---
version: 1
slug: "docs-design-widgets-mock-html"
primary_target: "docs/design/widgets-mock.html"
related_targets: []
---

# Surface brief — mock dos widgets (Fase 2)

## Escopo e modo

Mock de decisão em HTML estático (`docs/design/widgets-mock.html`) para o autor aprovar a interface dos widgets antes do PR 2 da Fase 2 (spec 0003 §2.9). Modo: Operate. Extensão do mundo B1 · Big Sur já registrado no `DESIGN.md`: mesmo palco 1:1 do mock da Fase 1, sem rodada de direção.

## Público, tarefa e prova

O autor, que usa o dock o dia inteiro, confere como Relógio, Progresso do tempo e Nota ficam no dock e nos painéis, em todos os estados que a spec pede: ícones vivos nos três tamanhos, temas e opacidades; painéis em modo normal e Ajustes; Configurações › Itens do dock; banco indisponível e recuperado; confirmação de remover nota; 12 widgets no tamanho Grande.

## Decisões confirmadas pelo usuário (2026-10-16)

- Relógio: as duas opções de ícone (digital e analógico), como ajuste de cada relógio.
- Progresso: anel com percentual no centro; no Pequeno, só o anel.
- Formato: igual ao mock da Fase 1 (palco 1:1, controles à esquerda, estados por URL).

## Restrições

- Sem desfoque real; painel sempre opaco; CSP estrita (tokens em classes estáticas, valores por item via CSSOM).
- Cobalto só para foco, seleção e painel aberto.
- Painel precisa caber acima do dock na área útil; nada de modal.

## Direction contract

THESIS: Os widgets são ladrilhos do próprio dock que mostram o valor vivo no esmalte (hora, anel do dia) e abrem um flyout do Windows 11 com o detalhe e os ajustes na mesma folha. Recusa a grade de cards de widget e o painel lateral de widgets do Windows.

OWN-WORLD: O mundo B1 inalterado: porcelana/grafite fosco, superelipse esmaltada (petróleo para o Relógio, musgo novo para o Progresso, ocre para a Nota), glifo e números brancos, painel opaco de 8 px, controle segmentado com véu de cobalto. Acréscimos: campo de texto do Windows 11, token de perigo argila-vermelho só para remoção, erro e selo.

STORY: O autor vê a hora e o dia passando no dock sem abrir nada, abre um widget com um clique, ajusta na mesma folha (Ajustes ↔ Voltar) e organiza a ordem em Configurações › Itens do dock, sabendo o que acontece quando o banco falha.

FIRST VIEWPORT: Barra de controles do mock à esquerda; palco 1:1 com desktop, janela do Explorer, barra de tarefas e o dock 8 px acima dela com Relógio, Progresso, Nota, separador, Configurações e Sobre; o painel aberto ancorado ao item. Abaixo, a folha de ícones vivos e as decisões/tokens.

FORM: Extensão do mundo fixado (B1 · Big Sur), sem sorteio. Assinatura: o ícone vivo (hora digital ou mostrador, anel do primeiro período). Seed key: none (pinned extension).

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
