# DockMax

[![CI](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml/badge.svg)](https://github.com/maxeinstein-dev/DockMax/actions/workflows/ci.yml)

Aplicativo desktop para Windows inspirado em docks personalizáveis, com foco em produtividade, widgets integrados e integração progressiva com o sistema operacional.

> **Status:** versão 0.3.0, o **MVP**: um dock que ocupa o lugar da barra de tarefas do Windows (com restauração garantida), nove estilos de barra, apps fixados com o ícone real e o menu do programa, relógio, bandeja do sistema, os widgets Relógio, Progresso do tempo e Nota, início com o Windows e dados locais em SQLite. O que ainda não existe (apps abertos, prévias, espaços de trabalho, mais widgets, ícones personalizados) está em [limitações](docs/limitacoes.md) e no [escopo do MVP](docs/roadmap.md). O MVP prioriza uso pessoal e demonstração de competências de arquitetura e desenvolvimento desktop. O DockMax é um projeto independente e não é afiliado ao PrimoDock.

## Instalar

Gere o instalador (`pnpm tauri build`, ver abaixo) e execute `src-tauri/target/release/bundle/nsis/DockMax_0.3.0_x64-setup.exe`. Ele instala **só para o seu usuário** (sem pedir administrador) e é **em português**. O instalador **não é assinado**: o SmartScreen do Windows avisa, e é preciso escolher "Mais informações › Executar assim mesmo". Em Configurações, "Iniciar com o Windows" abre o DockMax ao entrar. Para desinstalar, use "Aplicativos instalados" do Windows; os seus dados ficam, a menos que você peça para apagá-los.

## Objetivos

- Criar um dock personalizável e útil para o dia a dia.
- Usar React + TypeScript na interface e Tauri 2 + Rust na camada desktop/nativa.
- Manter arquitetura modular, testes e documentação de decisões.
- Evoluir gradualmente de launcher para recursos de gerenciamento de janelas.
- Manter os dados localmente sempre que possível.

## Documentos

- [`roadmap.md`](docs/roadmap.md): etapas, entregas, escopo do MVP e critérios de aceitação.
- [`limitacoes.md`](docs/limitacoes.md): o que o DockMax não faz hoje e o que pode surpreender.
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
pnpm tauri build          # build de release com o instalador NSIS
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
- **apps fixados** ([spec 0006](docs/specs/0006-fase-b-apps.md)): escolha na lista de programas instalados (a mesma do menu Iniciar) ou **solte um programa ou atalho no dock** para fixá-lo, com o ícone real; o clique abre o programa e o botão direito mostra o mesmo menu da barra do Windows (tarefas, recentes, Abrir, Remover); **arraste um app no dock** para mudar a ordem (ou use Configurações › Apps do dock);
- nove **estilos de barra** (Nativo, Liquid Glass, Meia-noite, Rocket, Prateleira Aqua, Neon, Canarinho, Papel e Flutuante), efeito ao passar o mouse e indicador, em Configurações › Aparência do dock;
- **relógio com data** no fim do dock e **bandeja do sistema** hospedada pelo DockMax: o botão direito num ícone abre o menu do próprio programa;
- **widgets**, cada um um item do dock que abre o próprio painel:
  - **Relógio**: hora viva no ícone (digital ou analógica), fuso, 12/24 h e segundos no painel;
  - **Progresso do tempo**: anel no ícone e painel com dia, semana, mês, ano e expediente;
  - **Nota**: texto com salvamento automático;
- painel de **Configurações** (tema, posição, tamanho, opacidade e **Itens do dock**, onde se adiciona, reordena e remove widgets) e painel **Sobre**;
- ícone na bandeja para mostrar ou ocultar o dock, abrir as Configurações e sair;
- **Iniciar com o Windows** ([spec 0008](docs/specs/0008-fechamento-do-mvp.md)), desligado por padrão;
- leitura de **pacotes de ícones** próprios da pasta `IconPacks` ([spec 0007](docs/specs/0007-fase-b4-icones.md)), como recurso avançado: há um pacote de exemplo em [`docs/examples/icon-packs`](docs/examples/icon-packs/).

Onde ficam os seus dados (nada sai da máquina):

| O quê | Onde |
|---|---|
| Preferências | `%APPDATA%\dev.maxsueleinstein.dockmax\settings.json` |
| Widgets e notas | `%APPDATA%\dev.maxsueleinstein.dockmax\dockmax.db` (SQLite) |
| Cópias de segurança e recuperação | `dockmax.db.bak-v*` e `dockmax.db.corrompido-*`, na mesma pasta: o DockMax nunca apaga seu banco nem esses arquivos, só renomeia um banco danificado |
| Pacotes de ícones (opcional) | `%APPDATA%\dev.maxsueleinstein.dockmax\IconPacks\<id>\` |
| Logs, sem dados pessoais | `%LOCALAPPDATA%\dev.maxsueleinstein.dockmax\logs` |

Depois do MVP, as próximas etapas seguem as fases da [spec 0004](docs/specs/0004-paridade-primodock.md): apps abertos e prévias de janela, espaços de trabalho e mais widgets. A personalização de ícones (por app, imagem própria, pacote embutido) ficou para o futuro por decisão do autor.

## Licença

[Uso pessoal](LICENSE) © 2026 Maxsuel Einstein. Todos os direitos reservados. As versões até a 0.2.0 foram publicadas sob a licença MIT.
