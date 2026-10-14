# Plano — Spec 0001 (Fase 0: repositório, toolchain e esqueleto)

- **Spec:** [0001-fase-0-esqueleto.md](0001-fase-0-esqueleto.md) (aprovada em 2026-10-14)
- **Execução:** cada tarefa é feita por um subagente com contexto limpo. Há uma revisão rápida entre batches e uma revisão completa (Fase 7 do /feature) no fim.
- **Commits:** um por tarefa, Conventional Commits em PT-BR, autor do usuário, sem `Co-Authored-By`.

## Legenda

- `[sequential]`: compartilha arquivo, contrato ou estado com outra tarefa, ou depende de uma anterior.
- `[parallel-safe]`: não compartilha arquivo com nenhuma outra tarefa do mesmo batch.

Cada batch é uma tarefa `[sequential]` ou um conjunto de tarefas `[parallel-safe]` sem sobreposição de arquivos.

## Batch 1 — Repositório

### T1 `[sequential]` — Inicializar git e commit inicial dos docs
**Spec:** §2.1 itens 1–4, 6.
**Arquivos:** `.git/`, `.gitattributes`.
- `git init -b main`; `git config core.autocrlf false` (local).
- `.gitattributes`: `* text=auto eol=lf` + binários (`*.png *.ico *.icns binary`).
- Commit em `main`: `docs: adiciona documentação inicial do projeto`, contendo `README.md`, `docs/architecture.md`, `docs/roadmap.md`, `docs/adr/*` e `.gitattributes`. Os arquivos de `docs/specs/` ficam de fora desse commit.
- `git remote add origin https://github.com/maxeinstein-dev/DockMax.git` (**sem push**).
- `git switch -c feat/fase-0-esqueleto`; commit `docs: adiciona spec e plano da fase 0` com `docs/specs/`.

**Pronto quando:** `git log main` tem 1 commit com o autor certo e sem trailer; a branch tem 2 commits.

## Batch 2 — Esqueleto

### T2 `[sequential]` — Gerar e enxugar o esqueleto Tauri + React
**Spec:** §2.2, §2.3.
**Arquivos:** `package.json`, `pnpm-lock.yaml`, `index.html`, `vite.config.ts`, `tsconfig*.json`, `src/**`, `src-tauri/**`, `.gitignore`, `.nvmrc`, `rust-toolchain.toml`.
- Gerar com `create-tauri-app` (template `react-ts`, gerenciador `pnpm`) **numa pasta temporária no scratchpad**, porque a raiz do repo não está vazia. Depois copiar para o repo sem sobrescrever `README.md` nem `docs/`.
- Remover a demonstração: logos, `App.css` de exemplo, `assets/`, comando `greet` (Rust e chamada `invoke`).
- Remover o plugin `opener` do `Cargo.toml`, do `package.json`, de `lib.rs` e de `capabilities/default.json`. Na capability fica só `core:default`.
- Mover o componente para `src/app/App.tsx` com o placeholder ("DockMax" + linha da fase).
- `tauri.conf.json`: `productName` "DockMax", `identifier` `dev.maxsueleinstein.dockmax`, `version` `0.1.0`, janela com título "DockMax", 800×600.
- Fixar `rust-toolchain.toml` (`1.99.0`, rustfmt + clippy), `.nvmrc` (`24`), `engines.node` e `packageManager: "pnpm@11.20.0"`.
- Sanidade: `pnpm install`, `pnpm build`, `cargo check` em `src-tauri/`.
- Commit: `feat: cria esqueleto Tauri 2 + React + TypeScript`.

**Pronto quando:** `pnpm build` e `cargo check` passam, e `rg -i "opener|greet"` fora de `docs/` e dos lockfiles não encontra nada.

## Batch 3 — Qualidade, CI e docs (paralelo)

As quatro tarefas não compartilham arquivos. O contrato entre T3a e T3c são os **nomes dos scripts**, fixados pela spec §2.4: `lint`, `format:check`, `typecheck`, `test`, `build`.

### T3a `[parallel-safe]` — Lint, formatação, tipos e testes do frontend
**Spec:** §2.4 (Frontend).
**Arquivos:** `package.json`, `pnpm-lock.yaml`, `tsconfig*.json`, `eslint.config.js`, `.prettierrc.json`, `.prettierignore`, `vitest.config.ts` (ou bloco `test` no `vite.config.ts`), `src/test/setup.ts`, `src/app/App.test.tsx` e formatação de `src/**`.
- `noUncheckedIndexedAccess: true` (o `strict` já vem no template).
- ESLint flat config: `typescript-eslint` recommended + `eslint-plugin-react-hooks` + `eslint-config-prettier`.
- Prettier ignora `docs/`, `*.md`, lockfiles e `src-tauri/`. Markdown fica fora para não reescrever os docs em PT-BR, e o Rust é do `rustfmt`.
- Vitest + RTL + jsdom; teste de fumaça que renderiza `App` e acha o texto "DockMax".
- Rodar `pnpm format`, depois confirmar que `lint`, `format:check`, `typecheck`, `test` e `build` passam.
- Commit: `chore: configura ESLint, Prettier e Vitest`.

### T3b `[parallel-safe]` — Checks do Rust
**Spec:** §2.4 (Rust).
**Arquivos:** `src-tauri/src/**`, `src-tauri/Cargo.toml` (só se o clippy exigir).
- `cargo fmt`; corrigir até `cargo clippy --all-targets -- -D warnings` e `cargo test` passarem.
- Commit (se houver diff): `style: aplica rustfmt e clippy no src-tauri`.

### T3c `[parallel-safe]` — Workflow de CI
**Spec:** §2.5.
**Arquivos:** `.github/workflows/ci.yml`.
- `windows-latest`, gatilhos `pull_request` e `push` em `main`.
- `actions/checkout`, `pnpm/action-setup` (lê `packageManager`), `actions/setup-node` (lê `.nvmrc`, cache pnpm), toolchain Rust pelo `rust-toolchain.toml`, cache de Rust para `src-tauri`.
- Ordem: install `--frozen-lockfile` → lint → format:check → typecheck → test → build → cargo fmt --check → clippy → cargo test.
- Actions fixadas por versão principal publicada (conferir a vigente ao escrever).
- Commit: `ci: adiciona workflow de verificação no Windows`.

### T3d `[parallel-safe]` — Licença e ADR 0005
**Spec:** §2.6 (LICENSE, ADR).
**Arquivos:** `LICENSE`, `docs/adr/0005-ferramentas-de-desenvolvimento.md`, `docs/adr/README.md`.
- MIT, "Copyright (c) 2026 Maxsuel Einstein".
- ADR no formato dos existentes (Status, Data, Contexto, Decisão, Alternativas, Consequências), cobrindo o que a spec §2.6 lista.
- Commit: `docs: adiciona licença MIT e ADR 0005`.

## Batch 4 — Documentação final

### T4 `[sequential]` — README, roadmap e status da spec
**Spec:** §2.6 (README, roadmap, spec). Depende dos batches 2 e 3 (versões reais, scripts, nome do workflow).
**Arquivos:** `README.md`, `docs/roadmap.md`, `docs/specs/0001-fase-0-esqueleto.md`.
- README: pré-requisitos com versões medidas, comandos de dev/build/checks, badge da CI, licença, "Estado atual".
- Roadmap: marcar só os itens da Fase 0 que foram verificados. "Criar repositório" fica marcado só depois do push.
- Commit: `docs: documenta ambiente de desenvolvimento da fase 0`.

## Batch 5 — Verificação local

### T5 `[sequential]` — Rodar a verificação da spec
**Spec:** Verificação 1–9 e 11.
- Do zero: `pnpm install --frozen-lockfile`, todos os checks de frontend e Rust, e `pnpm tauri build --no-bundle`.
- `pnpm tauri dev`: o usuário confere visualmente a janela (título, tamanho, placeholder).
- Registrar as saídas para o corpo do PR.

## Depois do plano (fases do /feature)

- **Fase 7 — Revisão** com `revisor-com-carta` sobre o diff inteiro da branch.
- **Fase 8 — Converge:** spec × código real, e checks rodados de novo.
- **Fase 9 — Ship:** push de `main` e da branch, PR e CI verde (Verificação 10), cada passo com confirmação explícita. Depois do merge: apagar a branch e marcar "Criar repositório" no roadmap.

## Rastreabilidade (spec → tarefa)

| Spec | Tarefa |
|---|---|
| §2.1 Repositório | T1 (push/PR na Fase 9) |
| §2.2 Esqueleto | T2 |
| §2.3 Versões | T2 (registro no README: T4) |
| §2.4 Frontend | T3a |
| §2.4 Rust | T3b |
| §2.5 CI | T3c |
| §2.6 LICENSE + ADR | T3d |
| §2.6 README, roadmap, spec | T4 |
| Verificação 1–9, 11 | T5 |
| Verificação 10 | Fase 9 |
