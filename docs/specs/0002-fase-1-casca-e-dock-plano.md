# Plano — Spec 0002 (Fase 1: casca do aplicativo e dock visual)

- **Spec:** [0002-fase-1-casca-e-dock.md](0002-fase-1-casca-e-dock.md) (aprovada em 2026-10-14)
- **Execução:** cada tarefa é feita por um subagente com contexto limpo. Há uma revisão rápida entre batches e uma revisão completa no fim de cada PR.
- **Commits:** um por tarefa, Conventional Commits em PT-BR, autor do usuário, sem `Co-Authored-By`.
- **Granularidade:** o PR 1 está detalhado. O mock, o PR 2 e o PR 3 estão em esboço e serão refinados **antes** de começar cada um. Eles dependem do resultado das validações V1–V3 e do mock, e o refinamento passa por uma checagem de "spec × plano × realidade" (fase 6 do /feature).

## Legenda

- `[sequential]`: compartilha arquivo, contrato ou estado com outra tarefa, ou depende de uma anterior.
- `[parallel-safe]`: não compartilha arquivo com nenhuma outra tarefa do mesmo batch.

## PR 1 — Janela (branch `feat/fase-1-janela`)

Quase tudo aqui mexe em `src-tauri/src/lib.rs`, `tauri.conf.json` e na capability. Por isso o PR 1 é **sequencial**: são validações empíricas em que cada passo depende do anterior funcionar no Windows real.

**Verificação visual sem depender só do usuário:** cada tarefa tira um print da janela via PowerShell (`System.Drawing`, só leitura da tela) e confere tamanho, posição e flags com `user32` (`GetWindowRect`, `GetWindowLongPtr`). A confirmação final continua sendo do usuário.

### J1 `[sequential]` — CSP
**Spec:** §2.1 item 1.
**Arquivos:** `src-tauri/tauri.conf.json`.
- Definir `app.security.csp` (e `devCsp`, se o hot reload do Vite exigir) sem `unsafe-eval` e sem `*`. Ponto de partida: `default-src 'self'; connect-src ipc: http://ipc.localhost; img-src 'self' asset: http://asset.localhost data:`, ajustado ao que o Tauri 2.12 injeta.
- Verificar em `pnpm tauri dev` e em `pnpm tauri build --no-bundle`: o placeholder renderiza (print), sem erro de CSP.
- Commit: `fix: define a Content Security Policy do app`.

### J2 `[sequential]` — Janela do dock
**Spec:** §2.1 itens 2, 3 e V3.
**Arquivos:** `tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/src/lib.rs`, novo `src-tauri/src/windows/dock.rs` (+ `mod.rs`), `src-tauri/Cargo.toml` (crate `windows`, se necessário), `src/` (placeholder do dock).
- Renomear a janela `main` para `dock` (e a capability junto): sem borda, transparente, sempre no topo, sem redimensionar, `skipTaskbar`, sem sombra do sistema, iniciando oculta até ser posicionada.
- Posicionar no **monitor principal**: centralizada na borda inferior da **área útil** (sem a barra de tarefas), com margem de 8 px lógicos. Recalcular em mudança de escala e de resolução.
- **V3 (Alt+Tab):** verificar se `skipTaskbar` já tira do Alt+Tab. Se não tirar, aplicar `WS_EX_TOOLWINDOW` via crate `windows`, isolado em `windows/dock.rs`, e documentar.
- Frontend: placeholder neutro do dock (dois botões sem estilo final: "Configurações" e "Sobre").
- Commit: `feat: cria a janela do dock posicionada acima da barra de tarefas`.

### J3 `[sequential]` — Janela de painel e IPC mínimo
**Spec:** §2.1 item 4.
**Arquivos:** `tauri.conf.json`, capabilities, `build.rs`, `src-tauri/src/lib.rs`, novos `src-tauri/src/windows/panel.rs` e `src-tauri/src/commands/panel.rs`, `src/` (rotas/entradas de dock e painel).
- Janela `panel` criada oculta, reaproveitada, sem borda, sempre no topo, fora da barra de tarefas.
- Comandos `toggle_panel(panelId, anchor)` (chamado pelo dock) e `hide_panel` (chamado pelo painel). O Rust calcula a posição a partir do retângulo do item e da área útil do monitor e emite `panel-show { panelId }` para a janela do painel.
- **Comandos com permissão explícita:** declarar os comandos do app no `AppManifest` do `tauri_build` e liberar cada um só para a janela que o usa (`dock` → `toggle_panel`, `panel` → `hide_panel`). Nenhuma janela ganha acesso a mais do que precisa.
- Fechar: Esc (frontend chama `hide_panel`), perda de foco (`WindowEvent::Focused(false)` → ocultar) e novo clique no item.
- **Armadilha conhecida:** clicar no item com o painel aberto faz o painel perder o foco (e se ocultar) **antes** do clique chegar, o que reabriria o painel. Tratar guardando qual painel foi ocultado por perda de foco e quando; um `toggle` do mesmo painel logo em seguida conta como fechar.
- Entrada do frontend separada por janela (`dock` e `panel`), sem roteador.
- Commit: `feat: adiciona janela de painel ancorada ao dock`.

### J4 `[sequential]` — Bandeja mínima e Alt+F4
**Spec:** §2.1 item 5.
**Arquivos:** `src-tauri/Cargo.toml` (feature `tray-icon`), `src-tauri/src/lib.rs`, novo `src-tauri/src/tray.rs`.
- Ícone do app na bandeja com "Mostrar/ocultar dock" e "Sair" (textos fixos em PT-BR nesta etapa; o `i18n.rs` chega no PR 2).
- `CloseRequested` na janela do dock: impedir o fechamento e ocultar.
- Commit: `feat: adiciona ícone na bandeja e oculta o dock no Alt+F4`.

### J5 `[sequential]` — V1: ampliação com clique atravessando a área transparente
**Spec:** §2.1 item 6 (V1).
**Arquivos:** `src-tauri/src/windows/dock.rs`, novo `src-tauri/src/windows/hit_test.rs`, `src/` (ampliação provisória no placeholder).
- Janela do dock mais alta que o dock visível (espaço para o ícone crescer).
- Janela começa ignorando o mouse. Uma tarefa no Rust acompanha a posição do cursor (~30–60 Hz) e liga a captura do mouse só quando o cursor entra no retângulo do dock visível; desliga ao sair.
- O frontend informa o retângulo do dock visível quando o layout muda.
- Medir: CPU em repouso com o acompanhamento ligado, latência perceptível ao entrar no dock e cliques na faixa transparente chegando ao app de trás (testar com um app real atrás).
- Critério de sucesso: clique na faixa transparente chega ao app de trás em 100% das tentativas manuais, e CPU em repouso abaixo de 1%. Se falhar, aplicar a alternativa da spec (sem ampliação), remover o acompanhamento e registrar.
- Commit: `feat: valida ampliação com clique atravessando a área transparente` (ou `docs: registra inviabilidade da ampliação e aplica alternativa`).

### J6 `[sequential]` — V2: vidro real com cantos grandes
**Spec:** §2.1 item 6 (V2).
**Arquivos:** `tauri.conf.json` (`windowEffects`), `src-tauri/src/windows/dock.rs`, `src/` (estilo provisório).
- Testar o efeito Acrylic na janela do dock e as formas de arredondar com raio grande: região da janela (`SetWindowRgn`) e preferência de canto do DWM. Conferir com print sobre um papel de parede contrastante.
- Testar lentidão ao reposicionar (mudança de borda) com Acrylic ligado.
- Se não houver forma de ter desfoque real com cantos grandes, remover o efeito e aplicar o **vidro simulado** (spec §2.1, V2).
- Commit conforme o resultado.

### J7 `[sequential]` — Medição, ADR 0006 e verificação do PR 1
**Spec:** §2.1 itens 7 e 9; Verificação 1–8.
**Arquivos:** `docs/adr/0006-janelas-do-dock-e-paineis.md`, `docs/adr/README.md`, `docs/specs/0002-fase-1-casca-e-dock.md` (linha de base de memória e resultados V1–V3).
- Memória em repouso: working set do processo e das WebViews (`msedgewebview2.exe` filhos) em release.
- ADR 0006: janelas separadas, permissões por janela, resultados de V1–V3 e comportamento de foco.
- Rodar toda a verificação do PR 1 e pedir a confirmação visual do usuário.
- Commit: `docs: registra decisões e medições da janela do dock`.

Depois do J7: revisão completa (`revisor-com-carta`), converge e PR, com push e PR só com aprovação do usuário.

## Mock visual (concluído: B1 · Big Sur escolhida; ver spec §2.2.1 e `DESIGN.md`)

### M1 `[sequential]` — Mock B1 × B2
**Spec:** §2.2. Depende do PR 1 (resultados V1/V2).
- Skill `impeccable`; arquivos em `docs/design/` (HTML estático + tokens).
- Publicar uma versão para você ver e escolher. A variante escolhida vira `docs/design/tokens.md` (ou CSS de tokens).
- Commit: `docs: adiciona mock visual da fase 1`.

## PR 2 — Fundação (branch `feat/fase-1-fundacao`)

Refinado em 2026-10-15, depois do mock (`DESIGN.md`) e com o código do PR 1 em `main`.

### Análise: spec × plano × realidade

| Achado | Ajuste |
|---|---|
| No esboço, i18n (F3) e error boundary (F5) mexeriam no mesmo `Panel.tsx`, e os tokens precisam adaptar `Dock.css`/`Panel.css` | Lanes redesenhadas sem arquivo compartilhado: o boundary nasce como componente isolado e é encaixado na integração |
| Módulo Rust só compila se declarado no `lib.rs`, e o clippy reprova código sem uso | A lane de configurações declara os próprios módulos com `allow(dead_code)` temporário; a integração remove o `allow` |
| Textos da bandeja (Rust) e do frontend estavam na mesma lane | `i18n.rs` vai para a integração Rust, que já mexe em `tray.rs` |
| Pendências da revisão do PR 1 (enum de layering, recorte dos retângulos de ampliação) | Lane própria (F7), só em `windows/` |
| A spec §2.2.1 pedia ao PR 2 a folga da janela no tamanho Grande | Pertence ao PR 3, onde o tamanho passa a mudar a janela; spec corrigida |
| `src/themes/base.css` tem o vidro provisório do PR 1 (`--glass-*`, raio 20) | Substituído pelos tokens do `DESIGN.md` (F4) |
| Spec §2.3.4 pede mensagem de erro no painel | O comando devolve o erro no PR 2; a mensagem na tela é do PR 3 (UI de Configurações) |

### Batch A — paralelo (sem arquivos compartilhados)

- **F1** `[parallel-safe]` **Testes-guarda** (spec §2.3.1). Arquivo novo `src-tauri/tests/guards.rs`: fixa o conteúdo exato de `capabilities/*.json` (lista de arquivos, janelas e permissões) e falha se a CSP de release for `null` ou tiver `unsafe-eval`/`*`. Bite-check de cada guarda.
- **F2** `[parallel-safe]` **Domínio e persistência de configurações** (§2.3.2–3). `src-tauri/src/domain/{mod,settings}.rs` (tipos, padrão, validação, presets 40/48/64 sobre `iconSize` 32–96), `src-tauri/src/persistence/{mod,settings_file}.rs` (leitura com fallback e gravação atômica), erros com `thiserror`; `Cargo.toml`/`Cargo.lock` (`thiserror`, `tempfile` em dev); declara os módulos no `lib.rs`. Testes: padrão, cada campo inválido, arquivo ausente, corrompido, versão desconhecida, gravação atômica (sem arquivo meio escrito), JSON compatível com o formato v1.
- **F3** `[parallel-safe]` **i18n do frontend** (§2.3.7). `src/i18n/` (`pt-BR.ts` como contrato de tipo, `useT()`, teste que prova que chave faltando quebra o `typecheck`), troca dos textos fixos em `src/dock/Dock.tsx` e `src/panel/Panel.tsx` (e ajuste dos testes deles).
- **F4** `[parallel-safe]` **Tokens do `DESIGN.md`** (§2.3.8). `src/themes/tokens.css` com os tokens dos dois temas (`[data-mode]`), tamanhos, raios, movimento e a paleta `.t-*`; `base.css` sem o vidro provisório; `Dock.css`/`Panel.css` passam a usar os tokens (sem redesenhar componentes, que é o PR 3); `src/dock/main.tsx`/`src/panel/main.tsx` importam os tokens. Sem `data-mode`, o tema segue `prefers-color-scheme`. Teste que confere os tokens do `DESIGN.md` presentes nos dois temas.
- **F5** `[parallel-safe]` **Error boundary** (§2.3.9). Componente novo `src/components/PanelBoundary.tsx` (estado de falha, reset ao trocar de painel) + teste. Ainda não encaixado.
- **F7** `[parallel-safe]` **Pendências do PR 1**. `windows/native.rs` (`Layering::Always`/`AsTaoSets` no lugar do `bool`, com teste que exige layered só no dock), call sites em `windows/dock.rs` e `windows/panel.rs`, e `windows/hit_test.rs` (recorte dos retângulos de ampliação pela janela, com teste).

### Batch B — integração Rust

- **F6a** `[sequential]` Comandos `get_settings`/`update_settings` (estado gerenciado, carga na inicialização com fallback e aviso), evento `settings-changed` para todas as janelas; `tauri-plugin-log` (arquivo rotativo, `info` em release e `debug` em dev, sem dados sensíveis) no lugar de todos os `eprintln!`, com a falha de captura registrada só na transição; `tauri-plugin-single-instance` (segunda execução mostra o dock e sai); `src-tauri/src/i18n.rs` para a bandeja; `build.rs` e capabilities mínimas (dock: `get_settings`; painel: `get_settings` + `update_settings`; os dois: `log` para o frontend); guarda do F1 atualizada; remove o `allow(dead_code)` do F2.

### Batch C — integração do frontend

- **F6b** `[sequential]` Hook `useSettings` (`src/core/settings.ts`) com `get_settings` + `settings-changed`; aplica o tema (`data-mode`) nas duas janelas, com "Sistema" seguindo o Windows em tempo real; encaixa o `PanelBoundary` no painel; logs do frontend pelo plugin.

### Batch D — fechamento

- **F8** `[sequential]` ADR 0007 (configurações em arquivo, validação no Rust, i18n tipado, logs); verificação 10–16 da spec no app real (persistência após reiniciar, arquivo apagado/corrompido, inspeção do log, instância única); revisão adversarial; PR.

## PR 3 — Dock e painéis (esboço; refinar depois do mock)

- Componentes do dock na variante escolhida (rótulo flutuante, ponto indicador, separador, ampliação se V1 passou).
- Painéis Configurações e Sobre; bandeja completa; dock em cima.
- Acessibilidade (teclado, rótulos, contraste AA, reduzir movimento).
- Roadmap da Fase 1 marcado; verificação 17–21; revisão e PR.
- Pendência herdada do PR 1: o `panel-show` é emitido antes de o listener do painel estar pronto na primeira abertura, e na troca de painel o conteúdo anterior aparece por um instante. Mostrar a janela só depois de o painel confirmar que renderizou o conteúdo pedido.
- Pendência da revisão do PR 2: hoje `update_settings` devolve a mensagem do erro em inglês. Antes de a tela de Configurações mostrá-la, o erro vira um código serializável, e o frontend traduz esse código por `pt-BR.ts`.
- Pendência da revisão do PR 2: "salvar a cada mudança" com um controle deslizante de tamanho gravaria o arquivo a cada passo, e o `sync_all` roda na thread principal (o comando é síncrono). A interface agrupa as mudanças (debounce) ou o comando passa a ser assíncrono.

## Rastreabilidade (spec → tarefa)

| Spec | Tarefa |
|---|---|
| §2.1.1 CSP | J1 |
| §2.1.2–3 Janela e posição; V3 | J2 |
| §2.1.4 Painel | J3 |
| §2.1.5 Bandeja e Alt+F4 | J4 |
| §2.1.6 V1 | J5 |
| §2.1.6 V2 | J6 |
| §2.1.7–9 Memória, visual provisório, ADR 0006 | J2 (visual), J7 |
| §2.2 Mock | M1 (concluído) |
| §2.3 Fundação | F1–F6b |
| §2.4 Dock e painéis | PR 3 |
| Verificação 1–8 | J1–J7 |
| Verificação 9 | M1 |
| Verificação 10–16 | F8 |
| Verificação 17–21 | PR 3 |
