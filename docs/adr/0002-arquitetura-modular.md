# ADR 0002 — Arquitetura modular com limites explícitos

- **Status:** Aceita como proposta inicial
- **Data:** 2026-10-14

## Contexto

O DockMax terá widgets com necessidades distintas: alguns são puramente visuais, enquanto outros acessam arquivos, sensores, mídia e APIs externas. Implementá-los de forma isolada e sem convenções tende a gerar duplicação e acoplamento.

## Decisão

Separar a aplicação em:
- interface React;
- núcleo e casos de uso;
- contratos/ports;
- adaptadores nativos e integrações;
- persistência.

O registro de widgets conterá metadados e componentes. Serviços com estado ou recursos de longa duração terão ciclo de vida explícito. Não exigir métodos de lifecycle de widgets que não precisam deles.

## Alternativas consideradas

- Um único conjunto de componentes com chamadas nativas espalhadas.
- Microserviços ou backend remoto.
- Um sistema de plugins dinâmicos desde a primeira versão.

## Consequências

- Mais disciplina na definição de contratos e erros.
- Testes podem substituir provedores nativos por mocks.
- O projeto permanece local e simples de operar.
- Abstrações serão adicionadas apenas quando houver necessidade concreta.
