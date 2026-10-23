# DockMax

[![CI](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml/badge.svg)](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml)

Aplicativo desktop para Windows inspirado em docks personalizáveis, com foco em produtividade, widgets integrados e integração progressiva com o sistema operacional.

> **Status:** em desenvolvimento rumo à paridade com o PrimoDock ([spec 0004](docs/specs/0004-paridade-primodock.md)). Já faz: dock no lugar da barra de tarefas do Windows (com restauração garantida), nove estilos de barra, apps fixados com o ícone real, relógio, bandeja do sistema, os widgets Relógio, Progresso do tempo e Nota, e dados locais em SQLite. O MVP prioriza uso pessoal e demonstração de competências de arquitetura e desenvolvimento desktop. O DockMax é um projeto independente e não é afiliado ao PrimoDock.

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
- [`adr/0006-janelas-do-dock-e-paineis.md`](docs/adr/0006-janelas-do-dock-e-paineis.md): janelas do dock e dos painéis, click-through e memória.
- [`adr/0007-configuracoes-logs-e-i18n.md`](docs/adr/0007-configuracoes-logs-e-i18n.md): configurações, logs e textos.
- [`adr/0008-persistencia-sqlite-e-widgets.md`](docs/adr/0008-persistencia-sqlite-e-widgets.md): SQLite, contrato de widgets e testes end-to-end.
- [`specs/`](docs/specs/): specs e planos de cada fase.

## Stack

- **Frontend:** React, TypeScript, Vite.
- **Desktop:** Tauri 2.
- **Integração nativa:** Rust e APIs compatíveis com Windows.
- **Persistência:** SQLite para dados estruturados e arquivos de configuração quando fizer sentido.
- **Testes:** Vitest, React Testing Library e testes Rust; testes end-to-end com WebdriverIO e `@wdio/tauri-service` (`pnpm test:e2e`, num build isolado que nunca toca nos seus dados; ver o [ADR 0008](docs/adr/0008-persistencia-sqlite-e-widgets.md)).
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

Testes end-to-end (a CI roda o job `e2e` com estes mesmos comandos). O build de teste usa uma identidade e uma pasta de dados temporária próprias: **não abre o seu DockMax nem toca nos seus dados**, mas mostra um segundo dock na tela por alguns segundos.

```bash
pnpm test:e2e:build       # gera o build de teste (feature e2e)
pnpm test:e2e             # roda os specs do WebdriverIO
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

O DockMax abre um dock que **ocupa o lugar da barra de tarefas do Windows**: reserva a borda da tela, esconde a barra nativa (por padrão; dá para deixá-la visível ou automática em Configurações) e a devolve exatamente como estava ao encerrar, de qualquer jeito que o programa termine ([ADR 0009](docs/adr/0009-dock-substitui-a-barra-de-tarefas.md)):

- ícones com ampliação, rótulo e ponto indicador; o clique atravessa a área transparente em volta do dock;
- **apps fixados** ([spec 0006](docs/specs/0006-fase-b-apps.md)): escolha na lista de programas instalados (a mesma do menu Iniciar), com o ícone real; o clique abre o programa; a ordem se muda em Configurações › Apps do dock;
- nove **estilos de barra** (Nativo, Liquid Glass, Meia-noite, Rocket, Prateleira Aqua, Neon, Canarinho, Papel e Flutuante), efeito ao passar o mouse e indicador, em Configurações › Aparência do dock;
- **relógio com data** no fim do dock e **bandeja do sistema** hospedada pelo DockMax: o botão direito num ícone abre o menu do próprio programa;
- **widgets**, cada um um item do dock que abre o próprio painel:
  - **Relógio**: hora viva no ícone (digital ou analógica), fuso, 12/24 h e segundos no painel;
  - **Progresso do tempo**: anel no ícone e painel com dia, semana, mês, ano e expediente;
  - **Nota**: texto com salvamento automático;
- painel de **Configurações** (tema, posição, tamanho, opacidade e **Itens do dock**, onde se adiciona, reordena e remove widgets) e painel **Sobre**;
- ícone na bandeja para mostrar ou ocultar o dock, abrir as Configurações e sair.

Onde ficam os seus dados (nada sai da máquina):

| O quê | Onde |
|---|---|
| Preferências | `%APPDATA%\dev.maxsueleinstein.dockmax\settings.json` |
| Widgets e notas | `%APPDATA%\dev.maxsueleinstein.dockmax\dockmax.db` (SQLite) |
| Cópias de segurança e recuperação | `dockmax.db.bak-v*` e `dockmax.db.corrompido-*`, na mesma pasta: o DockMax nunca apaga seu banco, só renomeia um banco danificado |
| Logs, sem dados pessoais | `%LOCALAPPDATA%\dev.maxsueleinstein.dockmax\logs` |

As próximas etapas seguem as fases da [spec 0004](docs/specs/0004-paridade-primodock.md) (menus dos apps, pacotes de ícones, espaços de trabalho, prévias e mais widgets); o [roadmap](docs/roadmap.md) ainda traz o plano anterior.

## Licença

[Uso pessoal](LICENSE) © 2026 Maxsuel Einstein. Todos os direitos reservados. As versões até a 0.2.0 foram publicadas sob a licença MIT.
