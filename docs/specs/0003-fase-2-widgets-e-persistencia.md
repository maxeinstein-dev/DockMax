# Spec 0003 — Fase 2: núcleo de widgets e persistência

- **Status:** Aprovada em 2026-10-15; em implementação (PR 1)
- **Data:** 2026-10-15
- **Roadmap:** [Fase 2 — Núcleo de widgets e persistência](../roadmap.md#fase-2--núcleo-de-widgets-e-persistência)
- **Origem das decisões:** sessão de grill de 2026-10-15 (Q1–Q16), resumidas no fim deste documento, e pesquisa de E2E e SQLite da mesma data.

## 1. O problema, medido

Estado em `main` após o PR maxeinstein-dev/DockMax#5 (fim da Fase 1, commit `a475d68`):

| Item | Estado medido |
|---|---|
| Itens do dock | Dois, fixos no código: `GROUPS = [["settings", "about"]]` em `src/dock/Dock.tsx`. |
| Painéis | `PanelId` é um enum fechado nos dois lados (`"settings" \| "about"` em `src/core/panels.ts`; `enum PanelId` em `src-tauri/src/windows/panel.rs`, com tamanho de reserva por painel). Não há como abrir o painel de algo criado em tempo de execução. |
| Persistência | Só `settings.json` v1 (ADR 0007): tema, borda, tamanho e opacidade. Nenhum banco, nenhuma migração, nada estruturado. |
| Widgets | Nenhum. Não há contrato de widget nem registro; o da arquitetura §5 é ilustrativo. |
| IPC | 6 comandos (`toggle_panel`, `hide_panel`, `panel_ready`, `set_dock_layout`, `get_settings`, `update_settings`), fixados por permissão em `capabilities/dock.json` e `panel.json` e travados por `src-tauri/tests/guards.rs`. |
| Testes | 144 testes Vitest e 188 testes Rust. Nenhum teste end-to-end: a ferramenta foi adiada para esta fase (spec 0002 §4). |
| Dependências Rust | `tauri`, `serde`, `serde_json`, `thiserror`, `log`, `tauri-plugin-log`, `tauri-plugin-single-instance`, `windows`. |

Nenhum critério de aceitação da Fase 2 é atendido hoje: não há instâncias, nem notas, nem configuração por widget.

## 2. O que esta spec entrega

A Fase 2 é entregue em **dois PRs**, com um **mock visual aprovado** entre eles. O PR 1 é a fundação, sem mudança visível na interface. O PR 2 traz os widgets.

### 2.1 Modelo de produto

1. **Cada instância de widget é um item do dock** (Q1). Clicar abre o painel daquela instância na janela de painel que já existe, ancorado ao item, com as mesmas regras de abrir e fechar da Fase 1.
2. **Ordem do dock** (Q12): os widgets ficam primeiro, na ordem escolhida. Depois vem o separador e, no fim, Configurações e Sobre, que são fixos e não podem ser removidos. Sem widgets, não há separador.
3. **Limite de 12 instâncias de widget**, validado no Rust.
4. **Primeiro uso:** o banco é criado com uma instância de Relógio, uma de Progresso do tempo e uma de Nota, nessa ordem. A semente roda **uma única vez**, na criação do banco: remover todos os widgets não os traz de volta.
5. **Tipos de widget** são um conjunto fechado, sem plugins de terceiros (arquitetura §1): `clock`, `time-progress`, `note`, `shortcut` e `files`. Nesta fase, só os três primeiros podem ser criados; `shortcut` e `files` existem só no domínio (§2.5).
6. **Rótulo opcional por instância** (até 40 caracteres), para distinguir dois relógios ("Lisboa", "São Paulo"). Sem rótulo, o item usa o nome do tipo. O rótulo aparece na pílula do dock e no título do painel.

### 2.2 Widgets

**Relógio** (Q5)
- O ícone do dock mostra a hora viva (`HH:mm`, sem segundos), legível no tamanho Pequeno (40 px) (Q2).
- O painel mostra hora, data e o fuso.
- Ajustes por instância:
  - fuso: o do sistema (padrão) ou um nome IANA;
  - formato 24 h (padrão) ou 12 h;
  - segundos no painel (padrão desligado).
- O nome do fuso é validado no Rust contra a base IANA. A interface formata com `Intl`. Se o WebView não conhecer um fuso que o Rust aceitou, o relógio usa o fuso do sistema e indica isso no painel, sem inventar a hora.

**Progresso do tempo** (Q6)
- Períodos: dia, semana, mês, ano e expediente. Para cada período o painel mostra percentual decorrido e tempo restante.
- O ícone do dock mostra um anel com o percentual do **primeiro** período da lista.
- Ajustes por instância:
  - quais períodos mostrar e em que ordem (ao menos um; padrão dia, semana, mês, ano);
  - início da semana: segunda (padrão) ou domingo;
  - horário de expediente: início e fim no mesmo dia, padrão 09:00–18:00, de segunda a sexta. O período "expediente" só aparece se estiver na lista. Fora do horário ou no fim de semana, ele mostra "fora do expediente", sem percentual.
- Dias com mudança de horário de verão usam a duração real do dia (23 h ou 25 h).

**Nota** (Q7)
- Texto simples, uma nota por instância, ícone estático.
- **Salvamento automático:** cerca de 500 ms depois da última digitação, ao perder o foco (Esc, clique fora, troca de painel) e antes de fechar o painel. Não há botão Salvar.
- **Limite de 64 KiB** (UTF-8), validado no Rust. Acima disso, o painel mostra a mensagem do erro e o texto não salvo continua na tela.
- O conteúdo da nota nunca vai para o log, nem em erro.
- **Remover uma nota com texto pede confirmação** ("Remover a nota apaga o texto"). Nota vazia sai sem perguntar (Q15).

### 2.3 Interface de instâncias

1. **Configurações › Itens do dock** (Q10): lista os widgets na ordem do dock. Tem botões **adicionar** (escolhendo o tipo), **subir** e **descer**, todos operáveis pelo teclado. Reordenar nunca muda o ID de uma instância. Arrastar e o menu de botão direito ficam para a Fase 3.
2. **Ajustes da instância dentro do próprio painel** (Q11): um botão "Ajustes" troca o conteúdo do painel para a configuração daquela instância (rótulo e campos do tipo). O botão **"Remover do dock"** fica ali também.
3. Como na Fase 1, cada mudança é aplicada e salva na hora, sem botão Salvar. O Rust é a fonte da verdade. Uma mudança aceita emite um evento para todas as janelas; numa falha, o valor volta ao anterior e o painel mostra a mensagem.
4. **Acessibilidade**, no nível da spec 0002 §2.4 item 7:
   - o `aria-label` do item é o rótulo ou o nome do tipo, nunca a hora;
   - o anel e a hora do ícone são decorativos, porque o painel tem a informação;
   - com "reduzir movimento", nada é animado.

### 2.4 Persistência

1. **SQLite para os dados estruturados, `settings.json` para as preferências globais** (Q3). Os dois convivem, e o `settings.json` existente não é migrado nem tocado.
2. **Banco** `dockmax.db` na pasta de dados do app (`%APPDATA%\dev.maxsueleinstein.dockmax`), aberto pelo Rust com `rusqlite` (SQLite embutido no binário).
   - O frontend nunca executa SQL: só chama comandos que validam a entrada.
   - Pragmas: WAL, `foreign_keys=ON` e `busy_timeout` de 5 s.
3. **Migrações versionadas** com `rusqlite_migration`: arquivos SQL em `src-tauri/migrations/`, embutidos no binário, versão em `PRAGMA user_version`. Cada migração roda numa transação.
4. **Modelo inicial** (migração 1):
   - `widget_instances`: `id` (UUID v7, texto), `kind`, `label` (opcional), `position`, `config` (JSON), `config_version`, `created_at`;
   - `notes`: `instance_id` (chave estrangeira com `ON DELETE CASCADE`), `body`, `updated_at`;
   - `meta`: chave-valor para estado interno do banco (por exemplo, o aviso de recuperação pendente).
5. **IDs estáveis:** UUID v7 gerado no Rust na criação, nunca reaproveitado e nunca alterado por reordenação, ajuste ou reinício.
6. **Configuração por widget validada no Rust** (Q4):
   - cada tipo tem um tipo Rust de configuração, com padrão e validação puros em `domain/`;
   - comandos rejeitam configuração inválida com um código de erro, sem caminho nem valor;
   - **configuração inválida lida do banco** (editada à mão ou de versão antiga): a instância usa o padrão do seu tipo, um aviso vai para o log com o tipo do problema, e o registro só é regravado quando o usuário muda algo. A inicialização nunca é bloqueada;
   - uma instância de tipo desconhecido é ignorada com aviso, sem apagar a linha.
7. **Recuperação** (Q9, versão híbrida):
   - **banco corrompido** (o SQLite diz que o arquivo não é um banco ou está corrompido): o arquivo é renomeado para `dockmax.db.corrompido-<data>`, **nunca apagado**. O app cria um banco novo, com a semente, e marca no `meta` um aviso de recuperação. A interface mostra o aviso (as notas antigas ficaram no arquivo renomeado) até o usuário dispensá-lo;
   - **migração que falha, ou banco de versão mais nova que o app:** o arquivo não é tocado. O dock mostra só Configurações e Sobre, e Configurações › Itens do dock explica que os widgets estão indisponíveis. O log registra só o tipo do erro;
   - **antes de cada migração** de um banco que já existe, uma cópia vai para `dockmax.db.bak-v<n>` (`VACUUM INTO`), onde `n` é a versão de antes;
   - o dock, as Configurações e o Sobre funcionam em qualquer um desses casos.
8. **Logs:** como no ADR 0007. Só a capacidade e o tipo do erro, nunca conteúdo de nota, rótulo, configuração ou caminho.

### 2.5 Estrutura de Atalho e Arquivos (Q8)

- Os tipos `shortcut` e `files` existem no domínio Rust, com configuração tipada e **validação sintática**, cobertos por testes:
  - **Atalho:** destino é um caminho absoluto do Windows (letra de unidade ou UNC, sem caractere nulo, até 32 767 caracteres) ou uma URL `http`/`https` com host;
  - **Arquivos:** uma pasta, com caminho absoluto do Windows.
- Nenhum acesso a disco ou rede: a validação só olha o texto. Abrir e listar chega na Fase 3, com o `opener`.
- Não aparecem na interface e não podem ser criados por comando nesta fase.

### 2.6 Contrato de widget e registro

- **Rust** (`domain/widgets/`): o enum de tipos, a configuração de cada tipo com padrão e validação, a instância, os erros com código e a regra "pode ser criado nesta fase".
- **TypeScript**:
  - `src/core/widgets.ts` espelha os tipos e embrulha os comandos, como `src/core/settings.ts`;
  - `src/widgets/registry.ts` liga cada tipo à sua apresentação: nome (chave do i18n), classe do esmalte do ícone, conteúdo do ícone, painel e ajustes.
- **Regras de negócio sem interface:** formatação do relógio, cálculo dos períodos de progresso e agenda do salvamento automático são funções TS puras, testadas no Vitest sem renderizar.
- **Contrato Rust × TS travado por teste:** o Rust compara a serialização de cada tipo com arquivos JSON de referência; o TS tem as mesmas fixtures tipadas (`satisfies`) e um teste que as compara com esses JSON. Mudar um lado sem o outro quebra um dos testes.
- A arquitetura §5 é reescrita com o contrato real, e o exemplo ilustrativo sai.

### 2.7 Comandos e eventos (forma)

Nomes e cargas finais saem do passo de contratos do plano. A forma decidida é esta:

- **Leitura (dock e painel):** lista ordenada de instâncias com o estado do armazenamento (disponível, recuperado ou indisponível).
- **Escrita (só painel):** adicionar, remover, mover para cima ou para baixo, mudar rótulo e configuração; ler e salvar nota; dispensar o aviso de recuperação.
- **Evento** `widgets-changed`, com a lista completa, para todas as janelas depois de uma mudança aceita.
- **`toggle_panel`** passa a aceitar a instância de um widget além de `settings` e `about`. O Rust confere que a instância existe.
- Cada comando novo exige permissão explícita na capability da janela que o usa, e o teste-guarda é atualizado.

### 2.8 E2E (Q14)

- **Ferramenta:** WebdriverIO com `@wdio/tauri-service` (provedor embutido), que encontra cada janela pela label.
- **Build de teste:**
  - `identifier` próprio (`dev.maxsueleinstein.dockmax.e2e`) e feature cargo `e2e`. Com isso o teste nunca acorda nem altera o DockMax do usuário, e o single-instance chaveia por essa `identifier`;
  - uma pasta de dados temporária por execução, aplicada com `appDirectoriesOverride`;
  - plugin de WebDriver e leitura da pasta temporária existem **só** com a feature `e2e`. O build normal não os contém.
- **Três testes:**
  1. o dock abre;
  2. trocar o tema aplica e persiste depois de reiniciar;
  3. criar uma nota, reiniciar e conferir o texto.
- **Onde roda:** num job próprio do CI (`windows-latest`) e localmente com `pnpm test:e2e`. Localmente, aparece um segundo dock na tela durante a execução, e o E2E só roda com autorização do usuário naquela hora.
- **Spike primeiro:** o teste 2 é feito antes de tudo no PR 1, rodando no CI. Se o provedor embutido falhar, as alternativas já estão decididas, nesta ordem: `tauri-driver` externo (mesmos testes) e Playwright por CDP.

### 2.9 Divisão em PRs (Q16)

**PR 1 — Fundação** (sem mudança visível na interface)
1. Spike do E2E e teste 2 no CI.
2. SQLite, migrações, backup antes de migrar e recuperação de banco corrompido.
3. Contrato e registro de widgets (Rust e TS), validação por tipo, incluindo o domínio de Atalho e Arquivos.
4. Comandos de instâncias e notas, evento, permissões e teste-guarda.
5. Regras de negócio TS puras (relógio, progresso e salvamento automático), com testes.
6. ADR 0008 e arquitetura §5 e §6 atualizadas.
7. Teste E2E 1.

**Mock visual** (Q13), um só, em `docs/design/`, seguindo o `DESIGN.md`. Mostra:
- os ícones vivos nos três tamanhos e nos dois temas;
- os três painéis, nos modos normal e Ajustes;
- Configurações › Itens do dock;
- os estados indisponível e recuperado, e a confirmação de remover nota.

Depois de aprovado, o `DESIGN.md` ganha os componentes novos.

**PR 2 — Widgets**
1. Relógio, Progresso e Nota completos.
2. Itens do dock, Ajustes e "Remover do dock".
3. Estados indisponível e recuperado na interface.
4. Dock com itens dinâmicos, separador e ícones vivos.
5. Testes E2E 1–3.
6. Roadmap da Fase 2 marcado conforme verificado.

## 3. O que esta spec deliberadamente NÃO entrega

- Abrir atalhos, arquivos, pastas e URLs; Atalho e Arquivos visíveis na interface: Fase 3.
- Arrastar para reordenar e menu de botão direito no dock: Fase 3.
- Widgets na área de trabalho e painel com vários widgets empilhados.
- Relógio mundial numa instância só (vários relógios são várias instâncias).
- Contagem regressiva até uma data.
- Markdown nas notas, várias notas numa instância, lixeira de notas.
- Migrar o `settings.json` para o SQLite.
- Sincronização, exportação ou importação de dados.
- Rolagem ou encolhimento do dock com mais de 12 widgets (Fase 4, junto com monitores e escala).
- Segundos no ícone do dock.
- Cobertura E2E além dos três testes.

## 4. O que esta spec deliberadamente NÃO decide (ainda)

- **Nomes e cargas exatas dos comandos e eventos:** saem do passo de contratos do PR 1, dentro da forma da §2.7.
- **Crate da base de fusos no Rust:** `jiff` é o candidato. Decidir no passo de contratos, pelo tamanho no binário e pelo tempo de build.
- **Visual dos ícones vivos, tamanhos dos painéis e do aviso de recuperação:** saem do mock.
- **Reinício no E2E** (`reloadSession` ou nova sessão): sai do spike.
- **Limite de memória:** a Fase 2 mede de novo a memória em repouso e compara com a linha de base da spec 0002, mas não fixa limite.

## Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| E2E instável no CI (WebView2, janelas ocultas, reinício) | Médio | Spike antes de tudo; alternativas em ordem já decididas. |
| E2E local alterando dados ou a tela do usuário | Alto | `identifier` próprio, pasta temporária, só com autorização. O build normal não contém o plugin. |
| Perda de notas por banco corrompido ou migração ruim | Alto | Nunca apagar; renomear o corrompido; backup antes de migrar; banco mais novo é só lido. |
| SQLite embutido aumentando o tempo de build e o binário | Baixo | Medir no PR 1; o rust-cache do CI reaproveita a compilação. |
| Hora errada no relógio por diferença entre a base de fusos do Rust e o `Intl` do WebView | Baixo | Fallback para o fuso do sistema, indicado no painel. |
| Timers dos ícones vivos consumindo CPU | Baixo | Um único timer no dock, alinhado à virada do minuto (arquitetura §9). |
| Dock largo demais com 12 widgets no tamanho Grande | Baixo | Limite de 12 validado; conferir em 1366 px no teste manual. |

## Verificação

Cada PR mostra a saída dos comandos, não a presume. Em todos os PRs: CI verde e os checks locais da spec 0001 (frontend e Rust, com `--locked`).

**PR 1 — Fundação**
1. Testes Rust de banco:
   - banco novo recebe a semente uma única vez;
   - migração em banco existente faz o backup `bak-v<n>`;
   - banco corrompido é renomeado e o app segue com banco novo e aviso;
   - banco de versão mais nova não é alterado e fica indisponível;
   - migração que falha não altera o arquivo;
   - `rusqlite_migration` valida as migrações.
2. Testes Rust de domínio: padrão e cada campo inválido de cada tipo, incluindo Atalho e Arquivos; limite de 12; limite de 64 KiB da nota; configuração inválida lida do banco vira padrão sem regravar.
3. Testes Rust de comandos e repositório: criar, remover, mover e ajustar mantêm os IDs; remover uma nota apaga o texto junto; a ordem sobrevive a reabrir o banco.
4. Teste de contrato Rust × TS falha ao mudar um lado só (bite-check mostrado).
5. Testes Vitest das regras puras:
   - relógio em 24 h e 12 h, em outro fuso, e com fuso desconhecido;
   - progresso em todos os períodos, incluindo virada de mês e ano, ano bissexto, dia de 23 h e 25 h, início da semana, expediente antes, durante, depois e no fim de semana;
   - agenda do salvamento automático (espera, reinício a cada tecla, salvamento imediato ao perder o foco).
6. Teste-guarda das capabilities falha ao ampliar uma permissão (bite-check) e passa com as permissões novas exatas.
7. E2E 1 e 2 verdes no CI e localmente (local só com autorização). Rodando com o DockMax do usuário aberto, o app do usuário não é afetado e o `settings.json` real fica igual (hash antes e depois).
8. O build de release não contém o plugin de WebDriver: `cargo tree` sem a feature `e2e` não lista o plugin.
9. Teste manual do usuário no build de release:
   - o app abre como antes;
   - o `dockmax.db` é criado ao lado do `settings.json`;
   - as preferências existentes continuam iguais.

**PR 2 — Widgets**
10. Critérios de aceitação da Fase 2 do roadmap:
    - instâncias têm IDs estáveis (mesmo ID depois de reordenar, ajustar e reiniciar);
    - configuração inválida não bloqueia a inicialização (banco editado à mão);
    - notas persistem depois de reiniciar (E2E 3 e teste manual);
    - a lógica de negócio é testável sem renderizar (itens 1–5).
11. Testes de componente: ícones vivos, painéis em modo normal e Ajustes, Itens do dock, estados indisponível e recuperado, e confirmação de remover nota.
12. Teclado: adicionar, subir, descer, Ajustes e remover operáveis sem mouse, com foco visível.
13. Contraste AA nos dois temas, nas duas opacidades e nos ícones vivos. Com "reduzir movimento", nada animado.
14. Memória em repouso medida com os três widgets e comparada com a linha de base da spec 0002.
15. Teste manual do usuário no build de release, no Windows real:
    - criar dois relógios com fusos diferentes, reordenar e reiniciar;
    - escrever uma nota, encerrar pela bandeja e reabrir;
    - remover a nota e confirmar;
    - dock com 12 widgets no tamanho Grande.

## Decisões da sessão de grill

| # | Decisão |
|---|---|
| Q1 | Cada instância de widget é um item do dock que abre seu painel. |
| Q2 | Relógio e Progresso com conteúdo vivo no ícone, a cada minuto; Nota com ícone estático. |
| Q3 | `settings.json` e SQLite convivem; nada migra (ADR 0008). |
| Q4 | Rust valida a configuração de cada tipo; TS espelha, com teste de contrato; regras de negócio em funções TS puras. |
| Q5 | Relógio: fuso do sistema ou IANA por instância, 24 h/12 h, segundos opcionais no painel. |
| Q6 | Progresso: dia, semana, mês, ano e expediente; início da semana; expediente padrão 09:00–18:00, desligado. |
| Q7 | Nota: texto simples, uma por instância, salvamento automático (~500 ms e ao perder o foco), 64 KiB, conteúdo nunca no log. |
| Q8 | Atalho e Arquivos só no domínio, com validação sintática; nada na interface. |
| Q9 | Banco corrompido: renomeia e recria, com aviso. Migração que falha ou banco mais novo: não toca, widgets indisponíveis. Backup antes de cada migração. |
| Q10 | Primeiro uso com Relógio, Progresso e Nota; Configurações › Itens do dock com adicionar, subir e descer. |
| Q11 | Ajustes e "Remover do dock" no painel de cada instância. |
| Q12 | Widgets primeiro, separador, Configurações e Sobre fixos no fim; até 12 widgets. |
| Q13 | Mock curto entre o PR 1 e o PR 2; `DESIGN.md` estendido. |
| Q14 | E2E com WebdriverIO + `@wdio/tauri-service`, build isolado, três testes, job no CI; local só com autorização. |
| Q15 | Remover nota com texto pede confirmação; sem lixeira. |
| Q16 | PR 1 fundação (spike de E2E primeiro), mock, PR 2 widgets. |
