# ADR 0004 — Windows primeiro e evolução da barra de tarefas

- **Status:** Aceita como proposta inicial
- **Data:** 2026-10-14

## Contexto

O objetivo é um aplicativo pessoal para Windows que possa evoluir em direção a um dock com capacidades semelhantes às de uma barra de tarefas. A substituição completa envolve detalhes de shell, janelas, foco, bandeja e múltiplos monitores.

## Decisão

O MVP será exclusivo para Windows e conviverá com a barra de tarefas nativa. A primeira versão oferecerá launcher, dock personalizável e painéis. Recursos de gerenciamento de janelas serão desenvolvidos e testados progressivamente.

## Alternativas consideradas

- Tentar substituir a barra de tarefas integralmente desde o primeiro protótipo.
- Projetar suporte Windows/macOS desde o início.

## Consequências

- O MVP pode ser entregue e usado antes de resolver todos os detalhes do shell.
- A compatibilidade com múltiplos monitores e tela cheia será avaliada durante a evolução.
- A arquitetura pode ser estendida, mas não promete portabilidade para macOS.
