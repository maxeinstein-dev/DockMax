# DockMax

Aplicativo desktop para Windows inspirado em docks personalizáveis, com foco em produtividade, widgets integrados e integração progressiva com o sistema operacional.

> **Status:** planejamento inicial. O MVP prioriza uso pessoal e demonstração de competências de arquitetura e desenvolvimento desktop. O DockMax é um projeto independente e não é afiliado ao PrimoDock.

## Objetivos

- Criar um dock personalizável e útil para o dia a dia.
- Usar React + TypeScript na interface e Tauri 2 + Rust na camada desktop/nativa.
- Manter arquitetura modular, testes e documentação de decisões.
- Evoluir gradualmente de launcher para recursos de gerenciamento de janelas.
- Manter os dados localmente sempre que possível.

## Documentos

- [`roadmap.md`](docs/roadmap.md): etapas, entregas e critérios de aceitação.
- [`architecture.md`](docs/architecture.md): arquitetura, componentes, dados e contratos.
- [`adr/README.md`](docs/adr/README.md): registro de decisões arquiteturais.
- [`adr/0001-stack-desktop.md`](docs/adr/0001-stack-desktop.md): stack.
- [`adr/0002-arquitetura-modular.md`](docs/adr/0002-arquitetura-modular.md): limites entre camadas.
- [`adr/0003-persistencia-local.md`](docs/adr/0003-persistencia-local.md): persistência.
- [`adr/0004-escopo-windows.md`](docs/adr/0004-escopo-windows.md): plataforma e barra de tarefas.

## Stack planejada

- **Frontend:** React, TypeScript, Vite.
- **Desktop:** Tauri 2.
- **Integração nativa:** Rust e APIs compatíveis com Windows.
- **Persistência:** SQLite para dados estruturados e arquivos de configuração quando fizer sentido.
- **Testes:** Vitest, React Testing Library e testes Rust; testes end-to-end a definir conforme a configuração Tauri.
- **Distribuição:** instalador Windows gerado pelo ecossistema Tauri.

As versões concretas das dependências serão fixadas no início da implementação, verificando compatibilidade e documentação oficial vigente.

## Princípios

1. Entregar valor em incrementos pequenos.
2. Não implementar todos os widgets antes de validar a base.
3. Não permitir que falha de um widget encerre o aplicativo inteiro.
4. Usar permissões mínimas para comandos e plugins.
5. Não expor segredos no frontend nem em logs.
6. Evitar abstrações sem necessidade comprovada.
7. Manter decisões técnicas relevantes em ADRs.

## Estado atual

Este repositório contém a documentação inicial, não o aplicativo implementado.
