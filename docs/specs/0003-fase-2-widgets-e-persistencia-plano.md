# Plano — Spec 0003 (Fase 2: núcleo de widgets e persistência)

- **Spec:** [0003-fase-2-widgets-e-persistencia.md](0003-fase-2-widgets-e-persistencia.md) (aprovada em 2026-10-15)
- **Execução:**
  - cada tarefa é feita por um subagente com contexto limpo;
  - as lanes paralelas rodam cada uma na sua worktree, criada a partir da branch do PR depois do passo de contratos, e voltam por merge;
  - há uma revisão rápida entre batches e uma revisão adversarial completa (`revisor-com-carta`, com bite-checks) no fim de cada PR.
- **Commits:** um por tarefa, Conventional Commits em PT-BR, autor Maxsuel Einstein, sem `Co-Authored-By`.
- **Granularidade:** o PR 1 está detalhado. O mock e o PR 2 estão em esboço e serão refinados **antes** de começar, com uma checagem de "spec × plano × realidade" (fase 6 do /feature), como no PR 2 da Fase 1.
- **Build Rust nas worktrees:** um `CARGO_TARGET_DIR` compartilhado no scratchpad, para não recompilar as dependências (SQLite embutido incluído) em cada lane. Builds simultâneos esperam a trava do cargo, o que é aceitável.

## Legenda

- `[sequential]`: compartilha arquivo, contrato ou estado com outra tarefa, ou depende de uma anterior.
- `[parallel-safe]`: não compartilha arquivo com nenhuma outra tarefa do mesmo batch.

## PR 1 — Fundação (branch `feat/fase-2-fundacao`)

O primeiro commit do PR leva esta spec e este plano.

### Batch 0 — risco primeiro

**E0** `[sequential]` **Spike do E2E** (spec §2.8)
- **Arquivos:**
  - `package.json` e `pnpm-lock.yaml`: WebdriverIO, `@wdio/tauri-service` e o script `test:e2e`;
  - pasta nova `e2e/`: `wdio.conf.ts`, `specs/theme.e2e.ts` e um `tsconfig` próprio;
  - `src-tauri/Cargo.toml` e `Cargo.lock`: plugin do WebDriver como dependência **opcional** e feature `e2e`;
  - `src-tauri/tauri.e2e.conf.json`, com a `identifier` de teste;
  - `src-tauri/src/lib.rs`, só em blocos `#[cfg(feature = "e2e")]`: plugin e `appDirectoriesOverride` a partir de `DOCKMAX_E2E_DATA_DIR`;
  - `.github/workflows/ci.yml`: job `e2e`;
  - `eslint.config.js`, `.prettierignore` e `tsconfig*.json`, se a pasta nova exigir.
- **O que fazer:**
  - build de teste com `pnpm tauri build --debug --no-bundle --features e2e --config src-tauri/tauri.e2e.conf.json`;
  - teste 2 da spec: abrir Configurações pelo dock, escolher Escuro, conferir `data-mode="dark"` no painel, reiniciar o app e conferir o tema no dock. Descobrir aqui se o reinício é por `reloadSession` ou por uma nova sessão;
  - provar o isolamento: a pasta temporária recebe `settings.json`, e um app normal aberto ao mesmo tempo não é acordado;
  - o job do CI precisa ficar verde antes de qualquer outra tarefa.
- **Se falhar:**
  - o provedor `external` (`tauri-driver`) fica com os mesmos testes;
  - se esse também falhar, o Playwright por CDP;
  - o resultado vai para o ADR 0008.
- **Local:** só executar com autorização do usuário naquela hora. Enquanto não houver autorização, a validação é pelo CI.
- **Commit:** `test: adiciona E2E com WebdriverIO e build isolado`.
- **Feito** em `f821c07`: o provedor embutido funcionou (resultado na spec §2.10).

### Batch A — contratos

**C1** `[sequential]` **Contratos da fundação** (spec §2.4–§2.7)
- **Arquivos:**
  - `src-tauri/Cargo.toml` e `Cargo.lock`: `rusqlite` (`bundled`), `rusqlite_migration`, `uuid` (`v7`, `serde`), a crate de fusos e `url`. Todas as dependências do PR entram aqui, para nenhuma lane mexer no `Cargo.toml`;
  - `src-tauri/migrations/0001_widgets.sql`: final, com `widget_instances`, `notes` e `meta`;
  - `src-tauri/src/domain/widgets/mod.rs`:
    - `WidgetKind`, `WidgetInstance` e `WidgetConfig` (enum por tipo);
    - as structs de configuração de cada tipo, com campos e serde finais;
    - `WidgetError` e `WidgetErrorCode`;
    - as constantes `MAX_WIDGETS = 12`, `MAX_NOTE_BYTES = 65536` e `MAX_LABEL_CHARS = 40`;
    - `decode_config(kind, json)`, que desserializa e chama `validate()`;
    - a regra `creatable()`;
  - `src-tauri/src/domain/widgets/{clock,time_progress,note,shortcut,files}.rs`: tipos e padrões, com `validate()` provisório que aceita tudo (a lane L2 implementa);
  - `src/core/widgets.ts`: tipos espelhados, nomes de comandos e eventos e cargas, como `src/core/settings.ts`, ainda sem o hook;
  - fixtures de contrato: `src-tauri/tests/fixtures/widgets/*.json` (padrão e um exemplo por tipo) e `src/core/widgets.fixtures.ts` (as mesmas, tipadas com `satisfies`), mais o teste Vitest que compara as duas.
- **Decisões a fechar aqui:**
  - nomes e cargas dos comandos (hoje na spec §2.7);
  - crate de fusos (`jiff`), medindo o tamanho do binário e o tempo de build antes e depois;
  - forma de `meta` e do estado do armazenamento (`ok`, `recovered` ou `unavailable`, com motivo).
- Declara os módulos novos com `allow(dead_code)` temporário, removido no I1.
- **Commit:** `feat: define o contrato de widgets e o esquema do banco`.
- **Feito** em `9a18fe4`: nomes da spec §2.7, `jiff` e `StorageStatus` com `ok`, `recovered` e `unavailable`.

### Batch B — paralelo (sem arquivos compartilhados)

- **L1** `[parallel-safe]` **Persistência**: `src-tauri/src/persistence/{mod,database,widgets_repo,notes_repo}.rs`.
  - Abrir o banco com os pragmas; migrar com `rusqlite_migration`; backup `bak-v<n>` (`VACUUM INTO`) antes de migrar um banco que já existe.
  - Banco corrompido: renomear para `.corrompido-AAAAMMDD-HHMMSS` (UTC), criar um novo e marcar o aviso no `meta`.
  - Banco de versão mais nova ou migração que falha: não tocar e devolver indisponível.
  - Semente única na criação.
  - CRUD de instâncias (posições contíguas, mover para cima e para baixo, limite de 12) e de notas (`ON DELETE CASCADE`).
  - Configuração que não decodifica vira padrão, com aviso no log e sem regravar.
  - Testes com pasta temporária para cada caso das verificações 1 e 3 da spec. A migração que falha é simulada com um conjunto de migrações de teste.
  - Commit: `feat: adiciona banco SQLite com migrações e recuperação`.
  - **Feito** em `dd373fc`.
- **L2** `[parallel-safe]` **Validação por tipo**: `src-tauri/src/domain/widgets/*.rs`.
  - Fuso IANA, formato, períodos (ao menos um, sem repetição), início da semana, expediente (início antes do fim, formato `HH:mm`), rótulo (até 40 caracteres, sem caractere de controle) e nota (até 64 KiB).
  - Atalho e Arquivos: caminho absoluto do Windows (unidade ou UNC, sem NUL, até 32 767 caracteres) e URL `http`/`https` com host, sem acesso a disco ou rede.
  - Teste de contrato que compara a serialização com `tests/fixtures/widgets/*.json`.
  - Testes de cada campo inválido; mensagens de erro sem caminho nem valor.
  - Commit: `feat: valida a configuração de cada tipo de widget`.
  - **Feito** em `15fcc80`.
- **L3** `[parallel-safe]` **Regras TS puras**: `src/widgets/clock/format.ts`, `src/widgets/time-progress/progress.ts` e `src/widgets/notes/autosave.ts`, cada um com teste.
  - Sem React e sem `invoke`; o relógio e o agendador são injetados, para testar sem esperar.
  - Casos da verificação 5 da spec.
  - Commit: `feat: adiciona as regras de relógio, progresso e salvamento da nota`.
  - **Feito** em `325bb38`.
- **L4** `[parallel-safe]` **Documentação**:
  - `docs/adr/0008-persistencia-sqlite-e-widgets.md`: SQLite ao lado do `settings.json`, recuperação, contrato Rust × TS e o resultado do E0;
  - `docs/adr/README.md`;
  - `docs/architecture.md`: §5 com o contrato real no lugar do ilustrativo, §6 e §12 com a ferramenta de E2E.
  - Commit: `docs: registra a persistência em SQLite e o contrato de widgets`.
  - **Feito** em `642a9ca`; alinhado ao código final depois da rodada 1.

### Batch C — integração (paralelo: Rust × TS)

- **I1** `[parallel-safe]` **Integração Rust**:
  - `src-tauri/src/commands/{mod,widgets}.rs`: comandos finos de listar, adicionar, remover, mover, ajustar, ler e salvar nota, e dispensar o aviso;
  - estado do armazenamento aberto no plugin `dockmax-startup`, depois das configurações (ADR 0007, ordem de inicialização);
  - evento `widgets-changed`;
  - `lib.rs`, com os comandos registrados e o `allow(dead_code)` removido;
  - `build.rs`, com os comandos no `AppManifest`;
  - `capabilities/dock.json`, que só lê;
  - `capabilities/panel.json`, que lê e escreve;
  - `tests/guards.rs`, com as permissões novas exatas e o bite-check mostrado.
  - Testes: conversão de erro em código, e o app abrindo com o banco indisponível.
  - Commit: `feat: expõe instâncias de widgets e notas por comandos`.
  - **Feito** em `c422f07`.
- **I2** `[parallel-safe]` **Integração TS**:
  - `src/core/widgets.ts`, com os wrappers dos comandos e o hook `useWidgets` (leitura + `widgets-changed`, no mesmo molde do `useSettings`);
  - `src/core/widgets.test.ts` com `mockIPC`;
  - `e2e/specs/dock.e2e.ts` (E2E 1: o dock abre com Configurações e Sobre).
  - Commit: `feat: adiciona o acesso do frontend às instâncias de widgets`.
  - **Feito** em `2f72ede`.

### Batch D — fechamento do PR 1

**V1** `[sequential]` **Verificação, revisão e PR**
- **Em andamento:** rodada 2 de revisão e teste manual. Medições já registradas na spec §2.10.
- Rodar todos os checks locais e mostrar a saída.
- Medir o tamanho do executável e o tempo de build com SQLite, e registrar na spec.
- E2E local, só com autorização; com o DockMax do usuário aberto, o hash do `settings.json` real é o mesmo antes e depois.
- Revisão adversarial (`revisor-com-carta`) do diff inteiro, com bite-checks nas guardas e na recuperação do banco.
- Converge: spec × código.
- Teste manual do usuário no build de release (verificação 9).
- Push e PR só com aprovação explícita.

**Rodada 1 de revisão** (três revisores independentes: persistência, IPC/logs/E2E, domínio/TS/docs). Bloqueou em três achados: companheiros órfãos apagados pelo SQLite, nomes de comandos e evento sem trava, e conjuntos fechados sem trava nos dois lados. As correções foram em quatro lanes:

- **F1 — Persistência** (`a9a04dd`, `d272aea`, `6ca42eb`, `08a2329`):
  - `.db` ausente ou vazio com `-wal`/`-shm`/`-journal` órfãos vira recuperação, e o `-journal` vai junto ao pôr um banco de lado;
  - semente e aviso de recuperação na mesma transação do esquema;
  - linhas ruins não derrubam a lista: UTF-8 inválido, rótulo inválido, tipo desconhecido ou não criável e ID ilegível;
  - `busy_timeout` de 250 ms, backup `bak-v<n>` reaproveitado, somente leitura como `openFailed`, log de leitura separado do de escrita e checkpoint `TRUNCATE` na saída.
- **F2 — Contrato e validação** (`9239ee8`, `7b94748`, `8d6cd0f`, `c9757cf`, `dd77f94`, `60bde8e`, `c179c20`):
  - `ipc.json` e `enums.json` travam comandos, argumentos, evento e conjuntos fechados nos dois lados (o `error-codes.json` saiu, coberto pelo `enums.json`);
  - rótulo invisível, com controle bidi ou com não-caractere recusado; `Factory` recusado no relógio;
  - URL do atalho exige `http://` ou `https://` e recusa `\` e `@` na autoridade; caminho recusa qualquer caractere de controle;
  - o erro da pasta de dados e de configuração não vai mais para o log;
  - documentação interna: rótulo em code points e `invalidConfig` também para carga não canônica.
- **F3 — Regras TS** (`c157102`, `1770fde`, `b0a3e0d`):
  - relógio não lança com fuso do sistema desconhecido;
  - autosave com prazo de 10 s por salvamento e código de erro só da lista conhecida;
  - progresso coberto com meia-noite inexistente e repetida em America/Santiago.
- **F4 — Guardas, E2E e CI** (`5d4c3f6`, `7a1645a`, `5e29017`):
  - guardas ignoram comentários, fixam o `tauri.e2e.conf.json` e aceitam só os dois arquivos de config do Tauri; `build.rs` × `generate_handler!` travados;
  - `assertTestBuild` exige a feature `e2e` no binário;
  - passo do CI que falha se o build sem a feature tiver o WebDriver.

Aceitos ou adiados: `config`/`body` gigantes desserializados antes do limite (aceito); ressincronizar o timer do dock em mudança de hora (PR 2); guarda Rust de logs com interpolação (proposta para depois). As pendências do PR 2 estão na spec §2.10.

## Mock visual (esboço)

**M1** `[sequential]` **Mock dos widgets** (spec §2.9), depois do merge do PR 1.
- Skill `impeccable`, seguindo o `DESIGN.md`, em `docs/design/widgets-mock.html`.
- Estados: ícones vivos (Relógio e anel do Progresso) nos três tamanhos e nos dois temas; painéis do Relógio, do Progresso e da Nota nos modos normal e Ajustes; Configurações › Itens do dock; indisponível; aviso de banco recuperado; confirmação de remover nota; dock com 12 widgets.
- Publicar para o usuário escolher. Aprovado, o `DESIGN.md` e os tokens ganham os componentes novos.
- Commit: `docs: adiciona mock dos widgets da fase 2`.

## PR 2 — Widgets (esboço, branch `feat/fase-2-widgets`)

- **W0** `[sequential]` **Contratos da interface**:
  - `PanelId` nos dois lados aceita a instância de um widget, e o `toggle_panel` confere que ela existe;
  - tamanho de reserva do painel por tipo;
  - todas as chaves de texto no `pt-BR.ts`, incluindo as mensagens dos códigos de erro;
  - `src/widgets/registry.ts`, com componentes provisórios nos caminhos finais de cada lane.
- **Batch paralelo**, uma pasta por lane:
  - **W1** Relógio: `src/widgets/clock/`, com ícone, painel e Ajustes;
  - **W2** Progresso: `src/widgets/time-progress/`;
  - **W3** Nota: `src/widgets/notes/`, com editor, salvamento automático, mensagem de limite e confirmação de remover;
  - **W4** Configurações › Itens do dock e os avisos de indisponível e recuperado (`src/panel/`).
- **W5** `[sequential]` **Integração**:
  - dock com itens dinâmicos, separador, ícones vivos com um timer único alinhado ao minuto, e largura com 12 widgets;
  - painel roteando para os widgets e para o modo Ajustes;
  - E2E 3 (nota sobrevive ao reinício);
  - roadmap marcado.
- **V2** `[sequential]` **Verificação, revisão e PR**:
  - verificações 10–15 da spec, com a memória medida;
  - revisão adversarial e converge;
  - teste manual do usuário no build de release;
  - push e PR só com aprovação.
