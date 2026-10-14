# ADR 0001 — Stack desktop

- **Status:** Aceita como proposta inicial
- **Data:** 2026-10-14

## Contexto

DockMax precisa de uma interface rica, animações e widgets, além de integração com APIs nativas do Windows. O projeto também deve aproveitar a experiência atual do autor com front-end e servir como projeto de portfólio.

## Decisão

Usar:
- React + TypeScript + Vite para a interface;
- Tauri 2 para o shell desktop e IPC;
- Rust para integração nativa e serviços locais;
- SQLite para dados estruturados;
- Vitest/React Testing Library e testes Rust.

## Alternativas consideradas

### Electron

Ecossistema amplo e familiar para desktop web, mas tende a ter maior consumo de recursos devido à arquitetura baseada em Chromium/Node. Pode ser reconsiderado se limitações concretas do Tauri bloquearem requisitos importantes.

### C# + WPF/WinUI

Integração natural com Windows, boa alternativa para um aplicativo exclusivamente Windows. Não foi escolhida inicialmente porque a combinação Tauri/React/TypeScript aproveita melhor a experiência de frontend que o projeto pretende desenvolver.

### Java desktop

Não é a melhor escolha para este caso, cujo núcleo é uma interface desktop e integração nativa com o Windows. Java/Spring não é necessário para o MVP local.

## Consequências

- O autor precisará aprender e manter código Rust.
- Alguns recursos exigirão APIs ou bibliotecas específicas do Windows.
- O conjunto de permissões Tauri deve ser configurado com cuidado.
- As versões exatas de dependências serão fixadas após validar os requisitos do ambiente.
