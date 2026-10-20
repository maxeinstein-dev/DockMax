# ADR 0007 — Configurações, logs e i18n

- **Status:** Aceita
- **Data:** 2026-10-15

## Contexto

O PR 2 da Fase 1 traz o primeiro código de domínio do DockMax: preferências que sobrevivem a reinicializações (tema, borda, tamanho e opacidade do dock), registro de falhas e a estrutura de textos da interface. O ADR 0003 prevê SQLite para dados estruturados, mas na Fase 1 só existe um pequeno conjunto de preferências. O app roda sem console no build de release, então falhas escritas com `eprintln!` ou `console.error` se perdiam.

## Decisão

### Configurações em arquivo, validadas no Rust

- Um `settings.json` versionado (`version: 1`) na pasta de configuração do app (`%APPDATA%\dev.maxsueleinstein.dockmax`), no lugar de SQLite nesta fase.
- O Rust é a fonte da verdade:
  - `domain/settings.rs` define tipos, padrão e validação, e é puro;
  - `persistence/settings_file.rs` lê e grava;
  - `commands/settings.rs` só adapta.
- **Leitura:** nunca bloqueia a inicialização. Arquivo ausente vira padrão. Arquivo corrompido (não é JSON) ou inválido (estrutura, valor ou versão) também vira padrão, com um aviso no log que diz só o tipo do problema. Campos ausentes recebem o padrão e campos desconhecidos são ignorados, para que um arquivo antigo continue válido. BOM UTF-8 é aceito, porque edições manuais no Windows o gravam.
- **Gravação atômica:** valida, escreve num temporário na mesma pasta, faz `sync_all` e renomeia sobre o destino. Em falha, remove o temporário e mantém o arquivo anterior.
- **Comandos:** `get_settings` e `update_settings`, este último com o objeto completo: ao contrário da leitura do arquivo, um campo ausente ou desconhecido é rejeitado, porque viria de um erro da interface e zeraria a configuração sem aviso. Uma mudança aceita emite `settings-changed` para todas as janelas. Em falha, o comando devolve só um código, `{ code }` (`iconSizeOutOfRange`, `unsupportedVersion`, `writeFailed` ou `notReady`), que a interface traduz pelo `pt-BR.ts`, sem caminho nem valor, e o estado e o arquivo ficam como estavam.
- **Permissões:** o dock só lê; só o painel grava (a tela de Configurações vive nele).
- Não há trait/port para o armazenamento: existe uma única implementação, e os testes usam pasta temporária (arquitetura §14).

### Logs em arquivo

- `tauri-plugin-log` com arquivo em `%LOCALAPPDATA%\dev.maxsueleinstein.dockmax\logs\dockmax.log`, girando a cada 1 MiB. Ficam o arquivo ativo e mais 4 girados, cerca de 5 MB no total (`KeepSome(4)`: na versão 2.10.1 do plugin, `KeepSome(n)` guarda n arquivos girados além do ativo).
- Níveis: o código do DockMax e o webview registram `Debug` no build de debug e `Info` no de release. As dependências registram só `Warn` ou acima, porque suas mensagens não foram revisadas quanto a dados pessoais.
- O frontend escreve no mesmo arquivo pelo plugin (`log:allow-log` nas duas janelas).
- **O log nunca impede o app de abrir.** O plugin é registrado sem logger, só pelo comando `log` das janelas, e o logger é montado no início do `setup` com `Builder::split`. Se o arquivo não abrir (travado durante a rotação, pasta sem permissão), o app segue com um logger sem o arquivo: stdout no debug, nenhum no release. Antes, essa falha derrubava o app inteiro no `build().expect` com `panic = "abort"`.
- **Regra de conteúdo:** uma linha de log nomeia a capacidade afetada e o tipo do erro (`settings: using the defaults: settings file is not valid JSON`, `panel: render failed: TypeError`). Nunca grava caminhos, conteúdo de configurações, argv, payloads nem a mensagem de um erro do frontend, que pode carregar qualquer um desses. No frontend, uma regra do ESLint (`no-restricted-syntax`, travada por `src/core/log.lint.test.ts`) aceita em `log.error|warn|info` só um literal de texto ou um template cujas interpolações sejam `errorKind(...)`.
- Falhas que se repetem a cada ciclo, como a captura do mouse a 60 Hz, são registradas só na transição de sucesso para falha e na recuperação.

### Ordem de inicialização

O logger e o carregamento das configurações rodam no setup de um plugin próprio (`dockmax-startup`), e não no `setup` do app. O Tauri cria as janelas do `tauri.conf.json` **antes** do `setup` do app, mas inicializa os plugins antes disso, no `Builder::build`. Com o carregamento no `setup`, a página do dock chamava `get_settings` antes de o estado existir e ficava sem tema no build de release; o painel, que carrega depois, escapava. Achado no teste manual do PR 2 e travado por teste-guarda.

### Instância única

`tauri-plugin-single-instance`, registrado primeiro: uma segunda execução mostra o dock da primeira (sem roubar o foco) e encerra. Argv e cwd são ignorados.

### i18n tipado, sem biblioteca

- **Frontend:** `src/i18n/pt-BR.ts` é a fonte e define o tipo `Messages`, com todas as folhas como `string`. Um idioma novo precisa cumprir o tipo: chave faltando ou sobrando quebra o `typecheck`, e um arquivo de contrato prova isso. `useT()` entrega o dicionário ativo, hoje só PT-BR.
- **Rust:** `src-tauri/src/i18n.rs` faz o mesmo para os textos da bandeja.
- Uma biblioteca (i18next etc.) só entra se aparecerem plurais ou formatação complexa.

### Tema e opacidade

- O frontend aplica as configurações como atributos no `<html>`:
  - `data-mode` só quando o tema é Claro ou Escuro;
  - `data-opacity` só na janela do dock, porque o painel é sempre opaco.
- Com "Sistema", nenhum atributo é posto: os tokens seguem `prefers-color-scheme`, e o WebView2 acompanha o tema do Windows ao vivo, sem script.
- A lista aberta de um `<select>` é uma janela própria do WebView2 (`Chrome_WidgetWin_1`, no processo do navegador) e pinta uma superfície clara em qualquer tema: ignora o `color-scheme` da página e do `<select>`, o fundo do `<select>` e o fundo das `<option>`, e não segue o `PreferredColorScheme` do perfil (o menu de contexto do mesmo perfil sai escuro enquanto a lista sai clara). Só a cor do texto das `<option>` é respeitada. Por isso `parts.css` dá às `<option>` a tinta do tema claro pelo token `--popup-ink` (um valor só, em `:root` de `tokens.css`, nunca sobrescrito nos blocos escuros), nunca `--ink`, e `parts.test.tsx` trava isso. Achado no reteste de release de 2026-10-19 (Windows escuro, tema "Sistema"), reproduzido pelo CDP com os cliques do usuário; `set_theme` da janela não é o remédio.
  - Risco inverso: se um WebView2 futuro passar a pintar o popup escuro (seguindo o `color-scheme`), as `<option>` com `--popup-ink` (escura) ficam ilegíveis sobre fundo escuro. O teste de `parts.test.tsx` não vê isso (o popup é nativo); vale conferir a lista no tema escuro a cada atualização do WebView2 e, se mudar, trocar `--popup-ink` por `--ink`.

### Isolamento de falhas por painel

O conteúdo de cada painel fica dentro de um error boundary (`PanelBoundary`) com `resetKey` igual ao id do painel. Uma falha de renderização mostra um alerta no lugar do conteúdo, mantém o título e o Esc funcionando, e trocar de painel recupera.

O React imprime no console, mesmo em produção, a mensagem e a pilha de todo erro de renderização, inclusive os capturados por um boundary. Por isso as raízes das duas janelas são criadas com `ROOT_OPTIONS` (`src/core/reactRoot.ts`): um erro capturado é registrado só pelo boundary (`onError`), e um erro não capturado ou recuperável vai para o log como `react: ...: <tipo>`, sem nada no console.

## Alternativas consideradas

- **SQLite já na Fase 1:** migrações e infraestrutura para quatro preferências. Fica para a Fase 2, junto com as notas (ADR 0003).
- **`tauri-plugin-store`:** chave-valor genérico em que o frontend grava qualquer chave, sem validação no Rust. Contraria "comandos validam a entrada" (arquitetura §2 e §8).
- **i18next/react-i18next:** biblioteca madura, mas com chaves em JSON sem checagem de tipo por padrão e mais dependência sem necessidade comprovada.
- **`tracing` montado à mão:** mais controle, mas sem rotação pronta e sem a ponte para o frontend que o plugin já traz.

## Consequências

- Preferências funcionam offline e sobrevivem a reinicializações. Uma mudança de formato no futuro exige migração versionada (`version`).
- Um `.tmp` pode sobrar na pasta de configuração se o app cair entre a criação e a renomeação; o arquivo de destino continua íntegro.
- Toda permissão nova continua passando pelo teste-guarda das capabilities.
- Testar o comportamento real do tema "Sistema" exige trocar o tema do Windows, o que é verificação manual.
