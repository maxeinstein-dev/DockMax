# DockMax

[![CI](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml/badge.svg)](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml)

Aplicativo desktop para Windows inspirado em docks personalizáveis, com foco em produtividade, widgets integrados e integração progressiva com o sistema operacional.

> **Status:** Fase 0 concluída: esqueleto do aplicativo compilando e abrindo localmente. O MVP prioriza uso pessoal e demonstração de competências de arquitetura e desenvolvimento desktop. O DockMax é um projeto independente e não é afiliado ao PrimoDock.

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
- [`adr/0005-ferramentas-de-desenvolvimento.md`](docs/adr/0005-ferramentas-de-desenvolvimento.md): ferramentas, versões e CI.
- [`specs/`](docs/specs/): specs e planos de cada fase.

## Stack

- **Frontend:** React, TypeScript, Vite.
- **Desktop:** Tauri 2.
- **Integração nativa:** Rust e APIs compatíveis com Windows.
- **Persistência:** SQLite para dados estruturados e arquivos de configuração quando fizer sentido.
- **Testes:** Vitest, React Testing Library e testes Rust; testes end-to-end a definir conforme a configuração Tauri.
- **Distribuição:** instalador Windows gerado pelo ecossistema Tauri.

Versões fixadas na Fase 0 (detalhes e motivos no [ADR 0005](docs/adr/0005-ferramentas-de-desenvolvimento.md)):

| Componente | Versão |
|---|---|
| Tauri | 2.12 (`tauri` 2.12.3, `@tauri-apps/cli` 2.12.2) |
| React | 19.3 |
| Vite | 8.3 |
| TypeScript | 6.0 |
| Rust | 1.99.0 (`rust-toolchain.toml`) |
| Node.js | 24 (`.nvmrc`) |
| pnpm | 11.20.0 (`packageManager`) |

As versões exatas de todas as dependências ficam travadas em `pnpm-lock.yaml` e `src-tauri/Cargo.lock`.

## Desenvolvimento

### Pré-requisitos (Windows 11)

- [Node.js](https://nodejs.org/) 24 e [pnpm](https://pnpm.io/) 11 (`corepack enable` instala a versão do `packageManager`).
- [Rust](https://rustup.rs/) via rustup, com toolchain MSVC. O `rust-toolchain.toml` instala a versão certa automaticamente.
- Visual Studio Build Tools com o workload **Desktop development with C++**.
- WebView2 Runtime (já vem no Windows 11).
- **Controle Inteligente de Aplicativos (Smart App Control) desativado.** Com ele ativo, o Windows bloqueia os build scripts do cargo (executáveis locais sem assinatura) e nada do lado Rust compila.

### Comandos

```bash
pnpm install              # dependências do frontend
pnpm tauri dev            # abre o app em modo desenvolvimento
pnpm tauri build          # build de release com instalador
pnpm tauri build --no-bundle  # só o executável, sem instalador
```

Verificações (as mesmas da CI):

```bash
pnpm lint
pnpm format:check
pnpm typecheck
pnpm test
pnpm build

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

## Princípios

1. Entregar valor em incrementos pequenos.
2. Não implementar todos os widgets antes de validar a base.
3. Não permitir que falha de um widget encerre o aplicativo inteiro.
4. Usar permissões mínimas para comandos e plugins.
5. Não expor segredos no frontend nem em logs.
6. Evitar abstrações sem necessidade comprovada.
7. Manter decisões técnicas relevantes em ADRs.

## Estado atual

Fase 0 concluída: o esqueleto Tauri 2 + React abre uma janela vazia e compila, e a CI verifica lint, formatação, tipos, testes e Rust a cada PR. A próxima etapa é a Fase 1 (casca do aplicativo e dock visual); veja o [roadmap](docs/roadmap.md).

## Licença

[MIT](LICENSE) © 2026 Maxsuel Einstein
