# Spec 0001 — Fase 0: repositório, toolchain e esqueleto

- **Status:** Implementada (verificação 10, CI verde no PR, pendente do push)
- **Data:** 2026-10-14
- **Roadmap:** [Fase 0 — Preparação e validação técnica](../roadmap.md#fase-0--preparação-e-validação-técnica)
- **Origem das decisões:** sessão de grill de 2026-10-14 (Q1–Q20), todas registradas abaixo.

## 1. O problema, medido

Estado do projeto e da máquina em 2026-10-14, antes de qualquer código:

| Item | Estado medido |
|---|---|
| Pasta `DockMax/` | Só documentação: `README.md`, `docs/architecture.md`, `docs/roadmap.md`, 4 ADRs. Todos UTF-8 com fim de linha LF. |
| Git local | Pasta **não** é repositório git. |
| Remoto `maxeinstein-dev/DockMax` | Existe, **público**, **vazio** (sem branch, sem licença). `gh` autenticado como `maxeinstein-dev`. |
| Identidade git global | `Maxsuel Einstein <maxsueleinstein.dev@gmail.com>`; `init.defaultBranch` e `core.autocrlf` não configurados. |
| Node / gerenciadores | Node 24.20.0, npm 12.0.2, pnpm 11.20.0. |
| Rust | rustc/cargo 1.99.0, toolchain `stable-x86_64-pc-windows-msvc`, com `clippy` e `rustfmt`. |
| Compilador C++ | Visual Studio Build Tools 2022 17.14 com `VC.Tools.x86.x64`. |
| WebView2 | 155.0.4283.45. |

O critério de saída da Fase 0 ("aplicação vazia abre localmente e pode ser compilada sem erros") ainda não pode ser verificado: não existe aplicação nem forma reproduzível de compilá-la.

## 2. O que esta spec entrega

### 2.1 Repositório e versionamento

1. `git init` na pasta, branch padrão `main`, `core.autocrlf=false` **local** ao repositório.
2. `.gitattributes` com `* text=auto eol=lf` (e binários comuns marcados como `binary`).
3. **Commit inicial em `main`** contendo apenas a documentação atual (sem alteração de conteúdo) + `.gitattributes`.
4. Todo o restante desta spec numa branch `feat/fase-0-esqueleto`, entregue via PR para `main`.
5. Commits em Conventional Commits, **mensagens em PT-BR**; identificadores e comentários de código em inglês.
6. Autoria: `Maxsuel Einstein <maxsueleinstein.dev@gmail.com>`, **sem** trailer `Co-Authored-By`.
7. Push, abertura de PR e merge só com confirmação explícita do usuário no momento de cada ação.

### 2.2 Esqueleto da aplicação

1. Gerado com o template oficial `create-tauri-app` (React + TypeScript + Vite, pnpm), Tauri **2.x**.
2. Demonstração do template removida: logos, CSS de exemplo, comando `greet` e qualquer chamada IPC. **Zero comandos Tauri** nesta fase.
3. Plugin `opener` removido (dependência Rust, pacote npm, registro em `lib.rs` e permissão). A capability da janela principal concede somente `core:default`.
4. Metadados: `productName` "DockMax", `identifier` **`dev.maxsueleinstein.dockmax`**, versão `0.1.0`.
5. Janela padrão do Tauri (com borda e barra de título), título "DockMax", 800×600, exibindo um placeholder simples (nome do app e uma linha indicando a fase).
6. Estrutura mínima, sem diretórios vazios (arquitetura §4):
   - `src/main.tsx`, `src/app/App.tsx` (+ teste e estilo mínimo, se necessário);
   - `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/capabilities/default.json`, `src-tauri/tauri.conf.json`, ícones padrão do template.

### 2.3 Versões fixadas

- `rust-toolchain.toml`: `channel = "1.99.0"`, `components = ["rustfmt", "clippy"]`.
- `.nvmrc`: `24`; `package.json` com `engines.node` restrito à linha 24 e `packageManager: "pnpm@11.20.0"`.
- Dependências npm e Rust nas versões vigentes no momento da instalação, travadas por `pnpm-lock.yaml` e `Cargo.lock` (ambos versionados).
- Versões principais resultantes registradas no README.

### 2.4 Qualidade: lint, formatação, tipos e testes

**Frontend**
- TypeScript `strict: true` + `noUncheckedIndexedAccess: true`. Sem `exactOptionalPropertyTypes`.
- ESLint (flat config) com regras recomendadas de TypeScript e `eslint-plugin-react-hooks`; Prettier para formatação, sem regras de estilo conflitantes no ESLint.
- Vitest + React Testing Library + jsdom; **um** teste de fumaça que renderiza `App` e verifica o placeholder.
- Scripts no `package.json`: `lint`, `format`, `format:check`, `typecheck`, `test`, além dos do template (`dev`, `build`, `tauri`).

**Rust**
- `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings`.
- Nenhum teste Rust artificial; `cargo test` deve passar com zero testes. O primeiro teste nasce com o primeiro código de domínio.

**Sem** hooks de pré-commit.

### 2.5 CI (GitHub Actions)

Um workflow em `windows-latest`, disparado em `pull_request` e em `push` para `main`, que:
1. instala Node pela `.nvmrc`, pnpm pelo `packageManager` e Rust pelo `rust-toolchain.toml`, com cache de dependências;
2. roda `pnpm install --frozen-lockfile`, `lint`, `format:check`, `typecheck`, `test` e `build` (Vite);
3. roda `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test` em `src-tauri/`.

O build do Vite precisa rodar antes dos passos Rust: `tauri::generate_context!` exige que `frontendDist` exista em tempo de compilação. A CI **não** gera instalador.

### 2.6 Documentação

- `LICENSE`: MIT, titular "Maxsuel Einstein", 2026.
- **ADR 0005 — Ferramentas de desenvolvimento**: pnpm, ESLint + Prettier (e por que não Biome), versões fixadas, CI mínima em Windows, ausência de hooks e de Dependabot de versões, identificador `dev.maxsueleinstein.dockmax` (e por que mudá-lo depois é caro). Índice em `docs/adr/README.md` atualizado.
- `README.md`: seção de desenvolvimento com pré-requisitos (versões medidas), comandos (`pnpm install`, `pnpm tauri dev`, `pnpm tauri build`, checks), badge da CI, licença; "Estado atual" atualizado.
- `docs/roadmap.md`: checkboxes da Fase 0 marcados conforme o que for de fato verificado.
- Esta spec, com status atualizado para "Implementada" ao final.

## 3. O que esta spec deliberadamente NÃO entrega

- Comportamento de janela de dock (sem borda, transparência, sempre no topo, posição, ocultação): **Fase 1**.
- Qualquer comando IPC, configuração persistida, tema ou tokens CSS: **Fase 1**.
- SQLite, migrações, contrato de widgets: **Fase 2**.
- Plugin `opener` ou qualquer abertura de arquivo/URL: **Fase 3**.
- Instalador, assinatura de código, empacotamento: **Fase 5**.
- Testes end-to-end.
- Hooks de pré-commit (husky/lefthook).
- `dependabot.yml` com atualizações de versão (os alertas de segurança padrão do GitHub continuam ativos).
- `exactOptionalPropertyTypes`.
- Testes Rust.

## 4. O que esta spec deliberadamente NÃO decide (ainda)

- **Ferramenta de E2E** para Tauri: decidir quando houver fluxo de usuário para testar (Fase 1/2).
- **Abordagem de CSS** (CSS Modules vs. CSS global com tokens): decidir na Fase 1, junto com os tokens de tema.
- **Se a CI gera instalador/artefato**: Fase 5.
- **Política de atualização de dependências** (Dependabot de versões ou manual): Fase 5.
- **Estratégia de releases/tags e changelog**: quando houver o primeiro instalador.
- **Content Security Policy** (`app.security.csp`, hoje `null`, padrão do template): definir no início da Fase 1, antes de qualquer widget renderizar conteúdo dinâmico. Na Fase 0 o risco é baixo: só o bundle local, sem IPC próprio e sem conteúdo remoto. Apontado na revisão da Fase 0.

## Riscos

| Risco | Mitigação |
|---|---|
| Primeira compilação do Tauri em `windows-latest` lenta (vários minutos) | Cache de `~/.cargo` e `src-tauri/target` na CI. |
| Template do `create-tauri-app` mudar de estrutura em relação ao esperado | A spec define o resultado (arquivos e permissões finais), não os passos do template. Divergências viram nota no PR. |
| `rust-toolchain.toml` com versão exata fica velho | Atualização deliberada e registrada; é o custo aceito de build reproduzível. |

## Verificação

Todos os passos a seguir devem passar, com a saída mostrada no PR ou na sessão, não presumida.

1. **Histórico**: `git log --format='%an <%ae>%n%b' main` mostra o commit inicial com o autor esperado e nenhum `Co-Authored-By`. Idem para os commits da branch.
2. **Instalação limpa**: `pnpm install --frozen-lockfile` conclui sem erro.
3. **Frontend**: `pnpm lint`, `pnpm format:check`, `pnpm typecheck`, `pnpm test` (1 teste passando) e `pnpm build` passam.
4. **Rust** (em `src-tauri/`): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` e `cargo test` passam.
5. **Compila**: `pnpm tauri build --no-bundle` gera `src-tauri/target/release/dockmax.exe` (ou o nome derivado do pacote) sem erros.
6. **Abre**: `pnpm tauri dev` abre uma janela com título "DockMax", 800×600, mostrando o placeholder. Confirmação visual do usuário.
7. **Permissões mínimas**: `src-tauri/capabilities/default.json` lista apenas `core:default`; `rg -i "opener|greet"` fora de `docs/` e dos lockfiles não encontra nada.
8. **Identificador**: `tauri.conf.json` contém `"identifier": "dev.maxsueleinstein.dockmax"` e `"productName": "DockMax"`.
9. **Versões fixadas**: `rust-toolchain.toml`, `.nvmrc`, `engines` e `packageManager` presentes com os valores da §2.3.
10. **CI**: o workflow fica verde no PR.
11. **Docs**: `LICENSE` (MIT), ADR 0005 indexado, README com comandos e versões, roadmap com a Fase 0 marcada.
