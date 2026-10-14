# ADR 0005 — Ferramentas de desenvolvimento

- **Status:** Aceita
- **Data:** 2026-10-14

## Contexto

A Fase 0 cria o esqueleto do DockMax e a forma de compilá-lo. Antes do primeiro código, é preciso definir gerenciador de pacotes, versões de toolchain, lint, formatação, testes, CI e convenções de commit. Algumas escolhas são baratas de trocar depois; outras, como o identificador do aplicativo, não são.

## Decisão

Versões e reprodutibilidade:
- pnpm 11 como gerenciador de pacotes, fixado via `packageManager`;
- Node 24 fixado via `.nvmrc` e `engines`;
- Rust fixado em 1.99.0 via `rust-toolchain.toml`, com `rustfmt` e `clippy`;
- `pnpm-lock.yaml` e `Cargo.lock` versionados.

Qualidade do frontend:
- ESLint (flat config, `typescript-eslint`, `eslint-plugin-react-hooks`) + Prettier;
- TypeScript com `strict` e `noUncheckedIndexedAccess`, sem `exactOptionalPropertyTypes`, pelo atrito com bibliotecas React;
- Prettier não formata Markdown/documentação, para não reescrever os documentos em PT-BR, nem Rust, que fica com o `rustfmt`;
- testes com Vitest + React Testing Library + jsdom.

Qualidade do Rust:
- `cargo fmt --check` e `cargo clippy --all-targets -- -D warnings`;
- nenhum teste Rust artificial até existir código de domínio.

CI e automação:
- um workflow mínimo do GitHub Actions em `windows-latest`, com lint, formatação, checagem de tipos, testes, build do Vite e `cargo fmt`/`clippy`/`test`, sem gerar instalador;
- sem hooks de pré-commit: a CI cobre as verificações com menos ferramentas;
- sem Dependabot de atualização de versões por enquanto; os alertas de segurança padrão do GitHub continuam ativos. Reavaliar na Fase 5.

Identificador e convenções:
- identificador do aplicativo `dev.maxsueleinstein.dockmax`, derivado do domínio do autor `maxsueleinstein.dev`. Ele define a pasta de dados do aplicativo em `%APPDATA%`, onde ficarão o SQLite e as configurações; por isso é decidido agora;
- commits em Conventional Commits, com mensagens em PT-BR; identificadores e comentários de código em inglês.

## Alternativas consideradas

### Biome

Uma única ferramenta para lint e formatação, mais rápida que ESLint + Prettier. Não foi escolhida porque ESLint + Prettier é o padrão de mercado que um revisor de portfólio reconhece, e as regras oficiais de hooks do React importam em um aplicativo cheio de timers e listeners. Continua sendo uma opção defensável no futuro.

## Consequências

- Builds reproduzíveis, ao custo de atualizar toolchain e dependências de forma deliberada e manual.
- Mudar o identificador depois deixa órfãos os dados já salvos na pasta antiga em `%APPDATA%`.
- Erros de formatação e lint só são barrados na CI, não no momento do commit.
- No Windows 11 com o Controle Inteligente de Aplicativos (Smart App Control) ativado, os build scripts do cargo, executáveis locais sem assinatura, são bloqueados (Code Integrity, os error 4551). A compilação local do Rust exige que esse recurso esteja desativado.
