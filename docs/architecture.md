# Arquitetura — DockMax

## 1. Contexto e objetivos

DockMax é uma aplicação desktop local para Windows, com interface em React/TypeScript e integração nativa via Tauri 2/Rust. Desde a [spec 0011](specs/0011-arquitetura-hibrida.md) a arquitetura é **híbrida**: o dock é desenhado em Rust nativo (Direct2D), sem WebView2, e os painéis e o app de Configurações são páginas React em janelas WebView2 criadas quando abrem.

Objetivos arquiteturais:

- Separar interface, regras de aplicação, persistência e integrações nativas.
- Facilitar a implementação incremental de widgets.
- Testar regras sem depender de APIs reais do Windows.
- Minimizar permissões e exposição de dados.
- Evitar infraestrutura remota desnecessária.

Não objetivos iniciais:

- Suporte multiplataforma.
- Backend web ou sincronização em nuvem.
- Substituição completa da barra de tarefas no MVP.
- Sistema de plugins de terceiros carregados dinamicamente.
- Implementação de todos os widgets na primeira versão.

## 2. Visão de alto nível

```text
React UI (WebView2, sob demanda)
  ├── Widget Panels
  ├── Settings / Themes
  └── Widget Components
          │
          ▼
Frontend Application Services
  ├── Widget Registry
  ├── Settings
  ├── Commands / Typed API
  └── Event Subscriptions
          │ Tauri IPC
          ▼
Rust Application Layer
  ├── Dock nativo (Direct2D, UI Automation)
  ├── Commands (thin adapters)
  ├── Domain / Use Cases
  ├── Ports (traits)
  ├── Persistence
  └── Adapters
       ├── Windows APIs
       ├── Hardware providers
       └── External services
```

### Regra de dependência

A interface depende dos contratos públicos, não da implementação interna de Windows ou SQLite. Os comandos Tauri validam entrada, chamam casos de uso e convertem erros em respostas seguras. A lógica de negócio não deve ficar dentro de handlers de comandos.

## 3. Stack

- React + TypeScript + Vite.
- Tauri 2.
- Rust.
- SQLite para dados estruturados, embutido no binário via `rusqlite` (`bundled`), com migrações por `rusqlite_migration`; preferências globais em `settings.json` (ADR 0007 e ADR 0008).
- CSS Modules ou CSS com tokens/variáveis bem definidos.
- Vitest e React Testing Library.
- Testes unitários Rust.
- Testes end-to-end com WebdriverIO e `@wdio/tauri-service`, sobre um build de teste isolado (ADR 0008).

Fixar versões reais na implementação e verificar requisitos atuais do Tauri, Rust e plugins antes de instalar.

## 4. Organização de diretórios sugerida

```text
src/
  app/
  components/
    dock/
    panels/
    settings/
    ui/
  widgets/
    registry.ts
    shortcuts/
    files/
    shelf/
    clock/
    time-progress/
    calendar/
    reminders/
    notes/
    music/
    battery/
    activity/
    fans/
    network/
    eyedropper/
    clipboard/
    drink-water/
    claude-usage/
    github/
    weather/
  core/
    widget-contracts/
    events/
    settings/
    errors/
  services/
    desktop/
    persistence/
    integrations/
  themes/
src-tauri/
  capabilities/
  migrations/
  src/
    commands/
    domain/
    ports/
    adapters/
      windows/
      hardware/
      integrations/
    persistence/
    lib.rs
    main.rs
tests/
  integration/
  e2e/
docs/
  adr/
```

Criar diretórios quando houver código real para colocar neles; não criar módulos vazios apenas para antecipar uma arquitetura futura.

## 5. Contrato de widgets

Cada **instância** de widget é um item do dock que abre o seu painel. Os tipos são um conjunto fechado, sem widgets de terceiros. O Rust é a fonte da verdade da forma e das regras; o TypeScript espelha os tipos e cuida da apresentação. Decisões e alternativas no [ADR 0008](adr/0008-persistencia-sqlite-e-widgets.md).

### Rust: `src-tauri/src/domain/widgets/`

Puro: não toca no banco, no Tauri nem no Windows. Um arquivo por tipo (`clock.rs`, `time_progress.rs`, `note.rs`, `shortcut.rs`, `files.rs`) com a configuração, o padrão e o `validate()`.

| Item | Papel |
|---|---|
| `WidgetKind` | `clock`, `time-progress`, `note`, `shortcut`, `files`, com o mesmo nome em kebab-case no JSON e na coluna `kind`. `creatable()` diz o que pode ser criado nesta fase (Atalho e Arquivos não, até a Fase 3). `config_version()` é a versão do formato da configuração, por tipo. |
| `WidgetConfig` | Enum com a configuração de cada tipo, marcado pelo tipo: `{ "kind": "clock", "config": { ... } }`. Uma configuração do tipo errado não chega a existir. `default_for(kind)` é o padrão e `validate()` aplica as regras que o serde não expressa. |
| `WidgetId` | UUID v7 em texto, gerado no Rust. Do frontend, só a forma exata que o Rust gera vira ID; qualquer outro texto é `notFound`. |
| `WidgetInstance` | O que a interface recebe: `{ id, label, kind, config }`. Sem posição: uma lista de instâncias está sempre na ordem do dock. `label` nulo usa o nome do tipo. |
| `WidgetsSnapshot` | O que `list_widgets` devolve e `widgets-changed` carrega: `{ storage, widgets }`. A lista só tem as instâncias visíveis (ver "Erros"). |
| `StorageStatus` | `ok`, `recovered` ou `unavailable` com o motivo (`newerVersion`, `migrationFailed`, `openFailed`). |
| `decode_config` / `encode_config` | Leem e gravam a coluna `config`. Campos ausentes recebem o padrão e desconhecidos são ignorados; outra versão ou JSON ilegível é `invalidConfig`. |

**Validação.** Limites de 12 instâncias e nota de até 64 KiB em UTF-8. Rótulo de até 40 code points (não unidades UTF-16: a interface conta com `[...label].length`), com ao menos um caractere visível e sem controle, separador de linha, controle bidi ou não-caractere. Por tipo:

- Relógio: fuso IANA conhecido pela base compilada no binário, na grafia exata da base; `Factory` recusado;
- Progresso do tempo: ao menos um período, sem repetição, e expediente `HH:mm` com o início antes do fim;
- Atalho e Arquivos: validação só sintática, sem acesso a disco ou rede. Caminho absoluto do Windows (unidade ou UNC) sem nenhum caractere de controle; URL que começa literalmente com `http://` ou `https://`, sem `\` e sem `@` na autoridade. Nomes de dispositivo do DOS ficam para a Fase 3.

**Erros.** Cada `WidgetError` tem um `WidgetErrorCode`, que é o que o comando devolve: `{ code }`, sem caminho nem valor. Códigos: `limitReached`, `notCreatable`, `notFound`, `kindMismatch`, `invalidLabel`, `invalidTimeZone`, `invalidPeriods`, `invalidWorkday`, `invalidTarget`, `invalidFolder`, `noteTooLarge`, `invalidConfig`, `unavailable` e `writeFailed`. A interface traduz cada um pelo `pt-BR.ts`.

**Comandos.** `list_widgets` (dock e painel). Só no painel: `add_widget`, `remove_widget`, `move_widget`, `update_widget` (rótulo e configuração completos; o tipo deve ser o da instância), `get_note`, `save_note` e `dismiss_storage_notice`. Uma mudança aceita emite `widgets-changed` para todas as janelas. A entrada é estrita:

- `kind` desconhecido é `notCreatable`; ID fora da forma exata é `notFound`;
- `update_widget` sem `label` é recusado pelo Tauri (`notReady` no TS), para um rótulo ausente não virar `null` e apagar o rótulo;
- a configuração de `update_widget` precisa ser a canônica, com todos os campos e nada mais (`invalidConfig`). Só a leitura do banco preenche campos ausentes com o padrão.

### TypeScript

- **`src/core/widgets.ts`:** espelha os tipos, as constantes de limite, os nomes de comandos e eventos e as cargas, no molde de `src/core/settings.ts`. Acrescenta o código `notReady`, para quando o comando nem chega a rodar.
- **`src/widgets/registry.ts`** (chega no PR 2 da Fase 2): liga cada tipo à sua apresentação — nome (chave do i18n), classe do esmalte do ícone, conteúdo do ícone, painel e ajustes. Não guarda regra nem valida configuração.
- **Regras de negócio em funções puras**, sem React e sem `invoke`, testadas no Vitest sem renderizar: formatação do relógio, períodos do Progresso e agenda do salvamento automático da Nota.
  - Relógio: um fuso que o `Intl` não conhece cai no do sistema, sinalizado; um fuso do sistema que o `Intl` não identifica (`Etc/Unknown`) formata sem `timeZone`. Nunca lança e nunca inventa um nome.
  - Salvamento automático: prazo de 10 s por salvamento. Um que não responde vira `writeFailed` e a próxima mudança tenta de novo. Ele pode ter sido gravado mesmo assim, então o texto guardado passa a ser desconhecido e a próxima mudança salva qualquer texto, mesmo o confirmado antes.

### Contrato travado por teste

Os JSON de referência ficam em `src-tauri/tests/fixtures/widgets/`. O Rust compara a serialização de cada tipo com eles. O TS tem as mesmas fixtures tipadas com `satisfies` (`src/core/widgets.fixtures.ts`), e `src/core/widgets.contract.test.ts` compara as duas. Mudar um lado sem o outro quebra um dos testes.

- `ipc.json`: nome do evento e de cada comando, com os nomes dos argumentos. Do lado Rust, conferido com as assinaturas dos comandos, o `AppManifest` do `build.rs` e o `generate_handler!` do `lib.rs`; do lado TS, com `WIDGET_COMMANDS`, `WIDGETS_CHANGED_EVENT` e os tipos dos argumentos.
- `enums.json`: todos os conjuntos fechados, na ordem do `ALL` de cada enum Rust, incluindo os códigos de erro.
- Um teste-guarda (`src-tauri/tests/guards.rs`) exige que `build.rs` e `generate_handler!` tenham exatamente os mesmos comandos, para todos os comandos do app.

### Ciclo de vida

- Uma instância nasce por `add_widget` (ou pela semente, na criação do banco), recebe o ID e a configuração padrão, e só sai por `remove_widget`. Remover uma Nota remove o texto junto.
- Reordenar, ajustar e reiniciar nunca mudam o ID.
- Widgets puramente visuais não têm métodos artificiais de inicialização e encerramento. Os que assinam eventos ou mantêm recursos usam hooks com cleanup explícito.
- Os ícones vivos (hora do Relógio, anel do Progresso) usam um único timer no dock, alinhado à virada do minuto, em vez de um por widget. Serviços de sistema de uso compartilhado também são centralizados, para evitar polling duplicado.

### Erros

- **Configuração inválida num comando:** rejeitada com o código do erro; o estado e o banco ficam como estavam, e o valor na interface volta ao anterior.
- **Configuração inválida lida do banco** (editada à mão ou de versão antiga): a instância usa o padrão do tipo, um aviso vai para o log com o tipo do problema, e a linha só é regravada quando o usuário muda algo. A inicialização nunca é bloqueada.
- **Linhas ocultas:** tipo desconhecido, tipo que não pode ser criado nesta fase (`shortcut`, `files`) ou ID ilegível. Ficam fora da lista com aviso, sem ser apagadas; mantêm a posição, não contam no limite e não podem ser movidas, ajustadas nem removidas. Mover uma instância visível pula as ocultas.
- **Texto que não é UTF-8** numa linha visível: rótulo vira nulo, configuração vira o padrão. Numa nota, `get_note` devolve `writeFailed`.
- **Armazenamento indisponível** (migração que falha, banco mais novo, arquivo somente leitura, banco que não abre): o arquivo não é tocado, o dock mostra só Configurações e Sobre, e Configurações › Itens do dock explica que os widgets estão indisponíveis.
- **Banco recuperado:** a interface avisa que um banco corrompido foi posto de lado, até o usuário dispensar o aviso.
- **Falha de um widget:** isolada no error boundary do painel; não encerra a aplicação.
- **Permissão negada:** explicar qual capacidade está indisponível.
- **Integração externa sem rede:** manter a UI responsiva e indicar dados desatualizados.

## 6. Persistência

Dois armazenamentos convivem, ambos lidos e gravados só pelo Rust. No Windows os dois ficam em `%APPDATA%\dev.maxsueleinstein.dockmax`:

- **`settings.json`** (ADR 0007), na pasta de configuração (`app_config_dir()`): preferências globais (tema, borda, tamanho e opacidade). Não migra para o SQLite, para as preferências carregarem mesmo com o banco indisponível.
- **`dockmax.db`** (ADR 0008), na pasta de dados (`app_data_dir()`): SQLite com instâncias de widget e notas.
  - Pragmas WAL, `foreign_keys=ON` e `busy_timeout` de 250 ms. Curto porque os comandos rodam na thread principal: com outra ferramenta segurando o banco, o salvamento falha rápido com `writeFailed` em vez de congelar o dock.
  - Migrações em `src-tauri/migrations/`, embutidas no binário, com a versão em `PRAGMA user_version`. Todas as pendentes rodam numa única transação.
  - Na saída do app, `wal_checkpoint(TRUNCATE)`: o processo termina sem fechar a conexão, e o `.db` precisa ficar completo sozinho.

Tabelas da migração 1 (`STRICT`):

| Tabela | Colunas | Observações |
|---|---|---|
| `widget_instances` | `id` (UUID v7), `kind`, `label` (opcional), `position`, `config` (JSON), `config_version`, `created_at` | `position` começa em 0 e é contígua. `config` não leva o tipo, que tem coluna própria. O frontend não vê `position`, `config_version` nem `created_at`. |
| `notes` | `instance_id`, `body`, `updated_at` | Chave estrangeira com `ON DELETE CASCADE`: remover a instância remove o texto. |
| `meta` | `key`, `value` | Estado interno do banco, como o aviso de recuperação pendente. Nunca dado do usuário. |

- **Semente única:** o banco nasce com Relógio, Progresso do tempo e Nota, na mesma transação do esquema. Não volta depois de remover todos.
- **Recuperação:** nenhum caminho apaga dados.
  - Banco corrompido, ou `.db` ausente ou vazio com companheiros órfãos com dados (`-wal` ou `-journal` com mais de 0 bytes; vazios ou um `-shm` sozinho não contam e dão um banco novo sem aviso): renomeado para `dockmax.db.corrompido-AAAAMMDD-HHMMSS` (UTC; `-2`, `-3`... se o nome existir), com `-wal`, `-shm` e `-journal` junto, e substituído por um novo, com aviso. O aviso é gravado na transação do esquema.
  - Migração que falha, banco mais novo que o app ou arquivo somente leitura: o arquivo não é tocado e os widgets ficam indisponíveis.
  - Antes de migrar um banco existente, `VACUUM INTO` grava `dockmax.db.bak-v<n>`, uma cópia por abertura. Uma cópia anterior nunca é substituída, e uma com os mesmos dados é reaproveitada.
- **Tabelas novas** (lembretes e outras) entram por migração nova, quando houver código que as use.

Não persistir métricas de CPU/RAM/rede indefinidamente. Começar com snapshots em memória; adicionar histórico apenas se houver um caso de uso concreto.

Configurações globais simples podem ficar em um arquivo de configuração. Não armazenar tokens ou segredos em texto puro no banco ou em JSON. Usar armazenamento seguro compatível com o ambiente.

## 7. Contratos nativos e serviços

Criar ports/traits Rust para capacidades que podem ter mais de uma implementação ou precisam ser mockadas:

- `AppLauncher`: abrir aplicações, arquivos, pastas e URLs.
- `WindowManager`: listar janelas elegíveis, ativar e minimizar quando tecnicamente suportado.
- `SystemMetricsProvider`: snapshots de bateria, CPU, memória e rede.
- `FanSensorProvider`: métricas de ventoinhas, com disponibilidade opcional.
- `ClipboardProvider`: acesso ao clipboard dentro das limitações do Windows e das permissões.
- `MediaSessionProvider`: metadados e controles de mídia compatíveis.
- `NotificationService`: notificações locais.
- `ExternalIntegration`: abstração por integração externa quando útil.

Não criar abstrações para toda função simples. Criar uma port quando ela isola uma API externa, permite teste, ou representa uma capacidade de domínio relevante.

## 8. Segurança

- Aplicar capacidades e permissões Tauri mínimas.
- Não habilitar acesso genérico a shell sem necessidade.
- Não aceitar comandos arbitrários da UI.
- Validar caminhos, URLs e payloads no lado nativo.
- Armazenar credenciais fora do frontend e protegê-las com mecanismo apropriado.
- Evitar registrar conteúdo do clipboard, notas, tokens e dados privados.
- Para GitHub/Claude/clima, usar autenticação oficial e respeitar limites e termos dos serviços.
- Não baixar nem executar código de widgets de fontes externas.

## 9. Desempenho e ciclo de vida

- Centralizar polling para recursos compartilhados.
- Permitir intervalo configurável por provedor.
- Pausar ou reduzir atualizações de widgets que não precisam de atualização constante.
- Não executar chamadas nativas em cada renderização.
- Cancelar listeners e timers quando o recurso deixa de ser usado.
- Medir consumo real antes de criar otimizações complexas.
- Medir a memória em repouso em todo PR de interface (processo + WebView2, build de release isolado, método da spec 0003 §2.12) e comparar com a linha de base e o orçamento do roadmap. Em repouso, com o dock nativo, não há WebView2 (spec 0011).

## 10. Dock e gerenciamento de janelas

O MVP convive com a barra de tarefas nativa. Evolução posterior:
1. Abrir aplicativos e destinos.
2. Enumerar janelas elegíveis.
3. Ativar e minimizar janelas.
4. Representar estado de aplicativos abertos.
5. Lidar com múltiplos monitores e escalas.
6. Investigar bandeja do sistema e outros comportamentos avançados.

Comportamento em tela cheia, foco, janela sempre no topo e ocultação automática precisam ser testados no Windows real. Não presumir que uma janela Tauri comum substitua as capacidades do shell do Windows.

**O dock nativo** (`src-tauri/src/windows/native_dock/`, spec 0011 H3) é uma janela Win32 em camadas, nunca ativada pelo clique, pintada com Direct2D e entregue por `UpdateLayeredWindow`: o clique atravessa onde ela é transparente. Vive na thread principal (a do laço de eventos do Tauri, que despacha as mensagens dela); as outras threads falam com ela por `post`, que enfileira o trabalho e acorda a janela. As regras são módulos puros com testes: a geometria (`domain/dock_layout.rs`), a lista de itens (`domain/dock_items.rs`) e o que os ladrilhos vivos mostram (`domain/live_faces.rs`). O que o dock pede ao resto do app (abrir um painel, um programa) roda depois de soltar o estado dele, porque pode voltar a ele. Para a acessibilidade, a janela responde à UI Automation como uma barra de ferramentas de botões.

## 11. Widget de uso do Claude

Tratar fontes como produtos de dados diferentes:
- Claude Code local: somente métricas encontradas em fontes locais suportadas.
- API: uso/custos se o usuário configurar credenciais válidas.
- Assinatura/organização: apenas dados expostos oficialmente para o tipo de conta.
- Analytics organizacional: respeitar requisitos de plano e permissões administrativas.

Exibir origem, horário de atualização e distinção entre dado oficial, estimativa e indisponibilidade. Nunca inferir limites de assinatura sem fonte confiável.

## 12. Testes

- Unitários TypeScript: validação de configuração, estado, formatação e lógica dos widgets.
- Componentes: estados normal, carregando, erro e indisponível.
- Unitários Rust: regras de domínio e conversão de erros.
- Contrato Rust × TS: fixtures JSON comparadas dos dois lados (§5).
- Integração: persistência/migrações e comandos com adaptadores simulados quando possível.
- E2E: abrir dock, mudar tema, criar nota, reiniciar e verificar persistência.
  - WebdriverIO com `@wdio/tauri-service`, provedor embutido: o servidor WebDriver roda dentro do app (`tauri-plugin-wdio-webdriver`) e encontra cada janela pela label.
  - Build de teste separado: feature cargo `e2e`, `identifier` `dev.maxsueleinstein.dockmax.e2e` (`src-tauri/tauri.e2e.conf.json`) e uma pasta de dados temporária por execução (`DOCKMAX_E2E_DATA_DIR`). O runner recusa um executável sem a `identifier` de teste ou sem a feature `e2e`.
  - O build normal não contém o plugin: um passo do job `checks` do CI falha se `cargo tree -e normal` listar `wdio` ou `webdriver`. Testes-guarda fixam o conteúdo do `tauri.e2e.conf.json` e aceitam só os dois arquivos de config do Tauri.
  - O dock nativo não tem página para o WebDriver: o build de teste tem uma janela oculta, `e2e`, de onde a suíte chama `e2e_dock_view` (o que o dock mostra) e `e2e_press_dock` (aperta um item), comandos que só esse build registra.
  - `pnpm test:e2e:build` e `pnpm test:e2e`. Roda num job próprio do CI (`e2e`) e, localmente, só com autorização do usuário, porque abre um segundo dock na tela.
  - Detalhes, reinício e alternativas no [ADR 0008](adr/0008-persistencia-sqlite-e-widgets.md).
- Manual: monitores, escala, tela cheia, permissões, instalação e inicialização automática.

## 13. Observabilidade

Logs locais com níveis e contexto técnico, sem conteúdo sensível. Erros de serviços devem identificar a capacidade afetada. Não enviar telemetria remota no MVP.

## 14. Critérios de evolução arquitetural

Antes de adicionar uma abstração, responder:
1. Qual problema real ela resolve?
2. Existe mais de uma implementação, ou precisamos de mock para testar?
3. Ela reduz acoplamento ou só aumenta indireção?
4. O custo de manutenção é menor que o benefício?

Revisar este documento quando decisões reais divergirem do plano e registrar decisões importantes em ADRs.
