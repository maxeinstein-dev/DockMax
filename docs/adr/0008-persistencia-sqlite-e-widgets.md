# ADR 0008 — Persistência em SQLite, contrato de widgets e E2E

- **Status:** Aceita
- **Data:** 2026-10-15
- **Spec:** [0003 — Fase 2: núcleo de widgets e persistência](../specs/0003-fase-2-widgets-e-persistencia.md)

## Contexto

A Fase 2 traz as primeiras instâncias de widget: cada uma é um item do dock, com ID estável, configuração própria e, no caso da Nota, um texto que o usuário não pode perder. Até aqui só existia o `settings.json` do [ADR 0007](0007-configuracoes-logs-e-i18n.md), com quatro preferências. O [ADR 0003](0003-persistencia-local.md) já previa SQLite para dados estruturados, sem dizer como abrir, migrar ou recuperar o banco.

Três perguntas ficaram para esta fase:

- onde guardar instâncias e notas, e o que fazer com o `settings.json`;
- como manter o Rust e o TypeScript de acordo sobre a forma de cada widget;
- como testar o app real de ponta a ponta, adiado desde a spec 0002 §4, sem tocar no DockMax do usuário.

## Decisão

### SQLite ao lado do `settings.json`

- **Banco** `dockmax.db` na pasta de dados do app (`%APPDATA%\dev.maxsueleinstein.dockmax`), a mesma do `settings.json`.
- Aberto pelo Rust com `rusqlite` (0.40, feature `bundled`). O SQLite é compilado dentro do binário: o app não depende de uma cópia do sistema e se comporta igual em toda máquina.
- **O frontend nunca executa SQL.** Só chama comandos que validam a entrada (arquitetura §2 e §8), como já acontece com as configurações.
- **Pragmas** em toda conexão: WAL, `foreign_keys=ON` (o SQLite não guarda essa opção no arquivo) e `busy_timeout` de 5 s, contra outra ferramenta abrindo o arquivo ao mesmo tempo.
- **Migrações** com `rusqlite_migration` (2.6):
  - arquivos SQL em `src-tauri/migrations/`, embutidos no binário;
  - versão em `PRAGMA user_version`, sem tabela de controle;
  - cada migração roda numa transação;
  - um teste chama a validação da própria crate sobre o conjunto.
- **O `settings.json` não migra e não é tocado.** Motivos:
  - ele já funciona, com gravação atômica e validação no Rust (ADR 0007);
  - migrar seria risco de perder preferências sem ganho para o usuário;
  - as preferências precisam carregar mesmo quando o banco falha: o dock, as Configurações e o Sobre funcionam com o banco indisponível.

### Modelo

Migração 1 (`src-tauri/migrations/0001_widgets.sql`), com tabelas `STRICT` para que o SQLite recuse um valor de tipo errado em vez de guardá-lo como outra coisa:

| Tabela | Colunas | Papel |
|---|---|---|
| `widget_instances` | `id`, `kind`, `label`, `position`, `config`, `config_version`, `created_at` | Uma linha por widget do dock. |
| `notes` | `instance_id` (FK com `ON DELETE CASCADE`), `body`, `updated_at` | Texto da Nota, no máximo uma linha por instância. Remover a instância remove o texto. |
| `meta` | `key`, `value` | Estado interno do banco, como o aviso de recuperação pendente (`recovery_notice` = `pending`). Nunca dado do usuário. |

- **ID:** UUID v7 em texto (minúsculo, com hífens), gerado no Rust na criação. Nunca reaproveitado e nunca alterado por reordenação, ajuste ou reinício. O v7 começa pelo instante de criação, então os IDs saem em ordem; em texto, ficam legíveis ao depurar. Do frontend, só vira ID o texto na forma exata que o Rust gera; qualquer outro é `notFound` e nunca chega ao banco.
- **Posição:** `position` começa em 0 e é contígua. O frontend não a vê: a lista que recebe já está na ordem do dock.
- **Configuração:** `config` guarda o JSON da configuração do tipo, sem o tipo (que tem coluna própria). `config_version` é a versão desse formato, por tipo; hoje todas são 1.
- **Semente única:** o banco nasce com Relógio, Progresso do tempo e Nota, nessa ordem. A semente roda só quando o banco é criado; remover todos os widgets não os traz de volta.
- **Limites** validados no Rust: 12 instâncias, rótulo de até 40 caracteres e nota de até 64 KiB em UTF-8.

### Recuperação: nunca apagar

| Situação | O que o app faz | Estado |
|---|---|---|
| Banco corrompido (o SQLite diz que não é um banco ou que está corrompido) | Renomeia o arquivo para `dockmax.db.corrompido-<data>`, cria um banco novo com a semente e marca o aviso no `meta`. A interface mostra o aviso até o usuário dispensá-lo. | `recovered` |
| Migração que falha | Não toca no arquivo. Os widgets ficam indisponíveis nesta execução. | `unavailable` (`migrationFailed`) |
| Banco de versão mais nova que o app | Não migra e não escreve. | `unavailable` (`newerVersion`) |
| Banco que não abre nem pode ser criado | Segue sem widgets. | `unavailable` (`openFailed`) |

- **Backup antes de migrar:** antes de cada migração de um banco que já existe, `VACUUM INTO` grava `dockmax.db.bak-v<n>`, onde `n` é a versão de antes. `VACUUM INTO` gera um arquivo único e consistente; copiar só o `.db` perderia o que ainda está no `-wal`.
- **Indisponível:** o dock mostra só Configurações e Sobre, e Configurações › Itens do dock explica que os widgets estão indisponíveis.
- **Logs:** como no ADR 0007. Só a capacidade e o tipo do erro, nunca conteúdo de nota, rótulo, configuração ou caminho.

### Contrato de widget: Rust valida, TS espelha

- **Tipos fechados:** `clock`, `time-progress`, `note`, `shortcut` e `files`. Não há widgets de terceiros (arquitetura §1).
- **O Rust é a fonte da verdade** (`src-tauri/src/domain/widgets/`, puro, sem banco, Tauri nem Windows):
  - `WidgetKind`, `WidgetConfig` (um enum com a configuração de cada tipo, marcado pelo tipo), `WidgetInstance`, `WidgetsSnapshot` e `StorageStatus`;
  - padrão e `validate()` por tipo;
  - `WidgetError`, com um `WidgetErrorCode` por erro. As mensagens não carregam caminho nem valor.
- **Configuração lida do banco** que não decodifica (outra versão, JSON ilegível, valor fora da regra): a instância usa o padrão do tipo, um aviso vai para o log com o tipo do problema e a linha só é regravada quando o usuário muda algo. Campos ausentes recebem o padrão e campos desconhecidos são ignorados, como no `settings.json`. A inicialização nunca é bloqueada. Uma linha de tipo desconhecido é ignorada com aviso, sem ser apagada.
- **Fuso do Relógio:** o nome é validado contra a base IANA compilada no binário (`jiff` sem os recursos padrão, com `tzdb-bundle-always`). Ele não lê o fuso do sistema nem a pasta de zoneinfo, então a resposta é a mesma em toda máquina.
- **TypeScript** (`src/core/widgets.ts`): espelha os tipos, os códigos de erro (mais `notReady`, só do TS) e os nomes de comandos e eventos, como `src/core/settings.ts`.
- **Contrato travado por teste dos dois lados:**
  - os JSON de referência ficam em `src-tauri/tests/fixtures/widgets/` (configurações de exemplo de cada tipo, os estados do armazenamento, uma lista e os códigos de erro);
  - o Rust compara a serialização de cada tipo com esses arquivos;
  - o TS tem as mesmas fixtures tipadas com `satisfies` (`src/core/widgets.fixtures.ts`), e `widgets.contract.test.ts` compara as duas;
  - mudar um lado sem o outro quebra um dos testes.
- **Regras de negócio em funções TS puras:** formatação do relógio, cálculo dos períodos do Progresso e agenda do salvamento automático da Nota. Ficam fora do React e do `invoke`, com relógio e agendador injetados, e são testadas no Vitest sem renderizar.
- **Atalho e Arquivos só no domínio até a Fase 3:** têm configuração tipada e validação sintática (caminho absoluto do Windows, URL `http`/`https` com host), sem acesso a disco ou rede. `creatable()` os recusa, então não podem ser criados por comando nem aparecem na interface.
- **Comandos:** `list_widgets` (dock e painel); `add_widget`, `remove_widget`, `move_widget`, `update_widget`, `get_note`, `save_note` e `dismiss_storage_notice` (só painel). Uma mudança aceita emite `widgets-changed` com a lista completa para todas as janelas. Em falha, o comando devolve só `{ code }`.

### E2E com WebdriverIO e build isolado

- **Ferramenta:** WebdriverIO 9.31 com `@wdio/tauri-service` 1.5, provedor `embedded`. O servidor WebDriver roda dentro do app, pela crate `tauri-plugin-wdio-webdriver` (1.5). Não há msedgedriver para manter na versão do WebView2.
- **Janelas pela label.** Com o provedor embutido, os handles do WebDriver são as labels do Tauri, então `browser.switchToWindow("panel")` basta. Isso importa porque as duas janelas têm o mesmo título.
- **Sem o `tauri-plugin-wdio`.** Ele exige `withGlobalTauri`, um import no frontend e uma capability própria, o que ampliaria o app normal. Só serviria para `browser.tauri.*`, que os testes não usam. O plugin de WebDriver não tem comandos IPC, então nenhuma capability muda.
- **Isolamento do DockMax do usuário:**
  - feature cargo `e2e`: o plugin e o módulo `src-tauri/src/e2e.rs` só existem com ela. O build normal não contém o plugin nem lê a variável de ambiente;
  - `src-tauri/tauri.e2e.conf.json` troca a `identifier` para `dev.maxsueleinstein.dockmax.e2e`. Com isso o teste tem trava de instância única, `%APPDATA%`, `%LOCALAPPDATA%` e perfil do WebView2 próprios, e nunca acorda o app do usuário;
  - o mesmo arquivo define `mainBinaryName` `dockmax-e2e`, para o executável de teste não se confundir com o `dockmax.exe` do `tauri dev` na mesma pasta;
  - `appDirectoriesOverride` aponta todas as pastas do app para `DOCKMAX_E2E_DATA_DIR`, uma pasta temporária nova por execução, criada no `onPrepare`;
  - o build de teste se recusa a abrir, antes de criar qualquer janela ou arquivo, se a `identifier` não terminar em `.e2e` ou se a pasta não for um caminho absoluto. O runner, por sua vez, recusa um executável que não seja o de teste.
- **Reinício:** `reloadSession()` sozinho não reinicia, porque o serviço abre o app uma vez e o reaproveita. O teste fecha as duas janelas, o app sai como no "Sair" da bandeja, um processo novo sobe com o mesmo ambiente e só então vem o `reloadSession()`. O app reiniciado é parado no `afterSession`: parado antes, no `after` do mocha, o runner não encontra o servidor para encerrar a sessão e o teste falha.
- **Servidor local:** o servidor WebDriver escuta em `127.0.0.1:4445` e aceita comandos de qualquer processo local enquanto o app de teste roda. Consequência aceita: ele só existe no build de teste, que só roda durante o E2E.
- **Onde roda:**
  - num job próprio do CI (`e2e`, `windows-latest`), com cache do Rust separado, porque a feature compila outro conjunto de dependências;
  - localmente com `pnpm test:e2e:build` e `pnpm test:e2e`, só com autorização do usuário naquela hora: aparece um segundo dock na tela durante a execução.
- **`pnpm-workspace.yaml` com `allowBuilds`:** o pnpm 11 usa `strictDepBuilds`, e sem a lista o `pnpm install --frozen-lockfile` falha por scripts de instalação ignorados. Os de `edgedriver` e `geckodriver` (baixam drivers que não usamos) e de `esbuild` ficam negados.
- **Resultado do spike (teste 2: tema persiste depois de reiniciar):**
  - passou localmente, com autorização, depois de mover a parada do app reiniciado para o `afterSession`;
  - o hash do `settings.json` real do usuário ficou igual antes e depois;
  - bite-check: reiniciando com a pasta de dados vazia, o teste falhou como esperado (o tema não volta);
  - o job `e2e` do CI: a medir no fechamento do PR 1.

## Alternativas consideradas

- **`tauri-plugin-sql`:** o frontend envia SQL, liberado por `sql:allow-execute`. A regra de negócio e o SQL ficariam no TS, contra "comandos validam a entrada".
- **`sqlx`:** assíncrono, com pool, e as macros checadas exigem `DATABASE_URL` ou cache offline. Pesado para um arquivo local de um usuário só.
- **Migrar tudo para o SQLite, inclusive as preferências:** acoplaria tema e tamanho do dock à saúde do banco, e um banco indisponível deixaria o app sem preferências.
- **`refinery` ou um runner de migração próprio:** `refinery` mantém tabela própria de histórico; um runner próprio é pouco código, mas a validação pronta pesou mais. Os dois usariam o mesmo `user_version`, então a troca futura não exige migrar dados.
- **UUID v4 ou ULID:** o v4 não tem ordem; o ULID exige outra crate e outro formato de texto. O v7 expõe o instante de criação, o que não importa num banco local.
- **`tauri-driver` externo + msedgedriver:** suportado pelo Tauri, mas o msedgedriver precisa casar a versão principal com o WebView2 da máquina, a falha clássica no CI. Fica como primeira alternativa, com os mesmos testes; as janelas seriam encontradas pela URL (`panel.html`), não pela label.
- **Playwright por CDP:** a própria documentação do Playwright descreve a conexão por CDP como de fidelidade menor, e o ciclo de vida do app ficaria à mão. Segunda alternativa.

## Consequências

- Instâncias e notas sobrevivem a reinicializações. Toda mudança de esquema é uma migração nova, com backup antes.
- Nenhum caminho de recuperação apaga dados. Arquivos `.corrompido-<data>` e `.bak-v<n>` podem se acumular na pasta do app; limpar fica com o usuário.
- O banco pode ficar indisponível sem derrubar o app: dock, Configurações e Sobre seguem funcionando.
- Mudar a forma de um widget exige mexer no Rust, no TS e nas fixtures juntos; os testes de contrato cobram isso.
- Uma configuração antiga ou editada à mão nunca bloqueia a inicialização, mas pode voltar ao padrão em silêncio para o usuário (o aviso fica só no log).
- Comandos novos continuam passando pela permissão por janela e pelo teste-guarda das capabilities (ADR 0006).
- O SQLite embutido e a base de fusos aumentam o executável e o tempo de build. Tamanho final e tempo: a medir no fechamento do PR 1.
- O E2E depende de um build de debug com a feature `e2e`; o build de release não muda. Rodar localmente abre um segundo dock na tela.
