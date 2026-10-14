# Spec 0002 — Fase 1: casca do aplicativo e dock visual

- **Status:** Aprovada; PR 1 implementado (resultados na §2.1.1)
- **Data:** 2026-10-14
- **Roadmap:** [Fase 1 — Casca do aplicativo e dock visual](../roadmap.md#fase-1--casca-do-aplicativo-e-dock-visual)
- **Origem das decisões:** sessão de grill de 2026-10-14 (Q1–Q34), resumidas no fim deste documento.
- **Referências visuais:** quadro de direções (A–D) apresentado na sessão; escolhida a **B (dock no estilo macOS)**, com identidade própria.

## 1. O problema, medido

Estado em `main` após o PR maxeinstein-dev/DockMax#1 (Fase 0):

| Item | Estado medido |
|---|---|
| Janela | Uma janela padrão do Tauri, com borda e barra de título, 800×600, placeholder estático. |
| IPC | Zero comandos Tauri. Capability `default.json` com somente `core:default`. |
| CSP | `app.security.csp: null` (padrão do template), adiada para esta fase pela spec 0001 §4. |
| Configurações / logs | Inexistentes. Nada é persistido; nada é registrado em arquivo. |
| Instância única | Inexistente: executar o app duas vezes abre duas janelas. |
| Interface | Textos em PT-BR fixos no componente; nenhuma estrutura de i18n. |
| Testes | 1 teste de fumaça (Vitest); 0 testes Rust. |
| Ambiente | Windows 11 build 26300, Smart App Control desligado, Rust 1.99.0, Tauri 2.12.3. |

Nenhum critério de aceitação da Fase 1 do roadmap é atendido hoje: não há dock, nem painéis, nem preferências que sobrevivam a reinicializações.

## 2. O que esta spec entrega

A Fase 1 é entregue em **três PRs**, com um **mock visual aprovado** entre o primeiro e o segundo. A ordem segue o risco: o que tem maior incerteza técnica (comportamento de janelas no Windows) é validado primeiro.

### 2.1 PR 1 — Janela (validação técnica)

1. **CSP definida** no primeiro commit, sem `unsafe-eval` e sem curingas (`*`), funcionando em `pnpm tauri dev` (inclusive o hot reload do Vite) e no build de release.
2. **Janela do dock**: sem borda, fundo transparente, sempre no topo, não redimensionável, **fora da barra de tarefas e do Alt+Tab**.
3. **Posicionamento**: monitor principal, **centralizada na borda inferior**, com margem acima da área útil (acima da barra de tarefas nativa). Reposiciona quando resolução ou escala mudam; verificado em 100%, 125% e 150%.
4. **Janela de painel**: uma única janela, criada oculta na inicialização e reaproveitada. Abre **ancorada ao item** clicado (acima do dock, ou abaixo quando o dock estiver em cima), contida na área útil do monitor, com tamanho fixo por painel. Fecha com **Esc**, com **perda de foco** e com **novo clique no mesmo item**. Um painel por vez.
5. **Bandeja mínima**: "Mostrar/ocultar dock" e "Sair". **Alt+F4** no dock oculta em vez de encerrar.
6. **Validações técnicas**, cada uma com alternativa já decidida:

   | # | Validação | Se não for viável |
   |---|---|---|
   | V1 | **Ampliação ao passar o mouse**: janela mais alta que o dock, com a área transparente deixando o clique passar (alternância dinâmica de "ignorar mouse" guiada pela posição do cursor, acompanhada pelo Rust). | Sem ampliação; o ícone sobe alguns pixels **dentro** do dock. |
   | V2 | **Vidro com desfoque real** (efeito Acrylic do Windows) **com cantos grandes** (~20 px). | **Vidro simulado**: camada translúcida mais opaca e levemente tingida, sem desfocar o fundo, mantendo os cantos grandes. Desfoque real com cantos de ~8 px está descartado. |
   | V3 | Dock **fora do Alt+Tab**. | Documentar a limitação e a chamada à API do Windows necessária, sem forçar. |

7. **Memória medida** (processo + WebViews do dock e do painel) em repouso, registrada nesta spec como linha de base.
8. Visual **neutro e provisório**; o visual final vem do mock.
9. **ADR 0006 — Janelas do dock e painéis**: janelas separadas, click-through, resultado das validações V1–V3.

### 2.1.1 Resultados do PR 1 (2026-10-14)

Detalhes e motivos no [ADR 0006](../adr/0006-janelas-do-dock-e-paineis.md).

| Item | Resultado |
|---|---|
| V1 — ampliação | **Viável.** Folga de 40 px com clique atravessando em repouso; captura por histerese guiada por `GetCursorPos` a 60 Hz; CPU em repouso ~0,03–0,16% de um núcleo. |
| V2 — vidro real com cantos grandes | **Inviável.** O backdrop do DWM ignora a região recortada e cobre a janela inteira. Adotado o vidro simulado em CSS, com opacidade num token. |
| V3 — fora do Alt+Tab | **Viável** com subclass Win32 que força `WS_EX_TOOLWINDOW` (o `skipTaskbar` do Tauri não basta). |
| CSP | Restritiva em release; em `tauri dev` no desktop não há CSP. |
| Memória em repouso (release, dock visível, painel oculto) | **182 MB privados** no total: `dockmax.exe` 6,4 MB + 7 processos do WebView2 176 MB. Working set somado: 388 MB (conta páginas compartilhadas mais de uma vez). Executável: 4,2 MB. |

### 2.2 Mock visual (entre o PR 1 e o PR 2)

- HTML estático em `docs/design/`, versionado, feito com o skill `impeccable`.
- Duas variantes da direção B, ambas com identidade **do próprio DockMax**, sem ligação com a marca pessoal do autor:
  - **B1 · Big Sur**: vidro claro/escuro, ícones em quadrado arredondado numa **paleta própria**, ponto indicador e rótulo flutuante em pílula. Referências: WhiteSur, Zorin OS, Dock do macOS.
  - **B2 · Prateleira**: o mesmo dock sobre uma prateleira de vidro com reflexo sutil e ampliação parabólica. Referências: Cairo-Dock, Latte Dock, Plank.
- Cada variante mostra: temas claro e escuro, os três tamanhos, opacidade translúcida e sólida, dock embaixo e em cima, painéis Configurações e Sobre, estados de hover, foco e ativo.
- Reflete as conclusões de V1 e V2 (por exemplo, sem ampliação se V1 falhar).
- Referências de terceiros são só visuais. Plank, Latte e Cairo-Dock são GPL e o DockMax é MIT: **nenhum código, ícone ou arquivo de tema é copiado**.
- **Saída**: variante escolhida + tokens de design (cores por tema e opacidade, raios, sombras, tipografia, tamanhos), que alimentam o PR 2. A identidade fica toda nos tokens, para poder mudar depois sem tocar em componentes.

### 2.3 PR 2 — Fundação

1. **Testes-guarda**, provados com bite-check:
   - fixa o conteúdo exato de `src-tauri/capabilities/*.json` (cada permissão nova exige mudar o teste, de forma visível no diff);
   - falha se `app.security.csp` for `null` ou contiver `unsafe-eval` ou `*`.
2. **Configurações**, primeiro código Rust real e modelo para os próximos módulos:
   - `domain/settings.rs`: tipos, valores padrão e validação, puros;
   - `persistence/settings_file.rs`: leitura e **gravação atômica** (arquivo temporário + renomear) em `settings.json` na pasta de configuração do app;
   - `commands/settings.rs`: comandos finos `get_settings` e `update_settings`, que validam, persistem e emitem o evento `settings-changed` para todas as janelas;
   - erros com `thiserror`, convertidos em mensagens seguras (sem caminhos de arquivo);
   - **sem trait/port** para armazenamento: há uma única implementação e o teste usa pasta temporária;
   - testes unitários Rust de validação, padrão, arquivo ausente, arquivo corrompido e gravação atômica.
3. **Formato `settings.json` v1**:
   ```json
   {
     "version": 1,
     "theme": "system",
     "dock": { "edge": "bottom", "iconSize": 48, "opacity": "translucent" }
   }
   ```
   - `theme`: `system` | `light` | `dark`; `dock.edge`: `bottom` | `top`; `dock.opacity`: `translucent` | `solid`;
   - `dock.iconSize`: número na faixa 32–96, com os presets Pequeno, Médio e Grande por cima (valores iniciais 40/48/64, ajustáveis pelo mock);
   - arquivo ausente, corrompido ou inválido: volta ao padrão, registra aviso no log e **não bloqueia a inicialização**;
   - ficam de fora de propósito: deslocamento ao longo da borda (sempre centralizado), estado oculto/visível (sempre abre visível), posição em pixels (derivada) e idioma (entra como `locale` com padrão `pt-BR` quando o inglês existir, sem migração).
4. **Fluxo de gravação**: a interface aplica e salva **a cada mudança**, sem botão Salvar. O Rust é a fonte da verdade. Em falha de gravação, o comando retorna erro, o painel mostra a mensagem e o valor volta ao anterior.
5. **Logs** com `tauri-plugin-log`: arquivo rotativo na pasta de logs do app, nível `info` em release e `debug` em dev, frontend e Rust no mesmo arquivo. Nunca registrar conteúdo de configurações, caminhos do usuário ou payloads; só a capacidade afetada e o tipo de erro.
6. **Instância única** com `tauri-plugin-single-instance`: a segunda execução mostra o dock da primeira e encerra.
7. **i18n estruturado, só PT-BR**:
   - `src/i18n/pt-BR.ts` define as mensagens; o tipo delas é o contrato que um futuro `en.ts` terá de cumprir (o build falha se faltar chave);
   - hook `useT()` para os componentes; datas e números via `Intl`;
   - `src-tauri/src/i18n.rs` equivalente para os textos da bandeja.
8. **Tokens de tema** vindos do mock, como variáveis CSS, com tema Sistema seguindo a troca do Windows em tempo real.
9. **Error boundary por painel**: um painel que lança erro mostra estado de falha sem afetar o dock nem os outros painéis. Provado em teste de componente.
10. **ADR 0007 — Configurações e i18n**: arquivo de configuração no lugar de SQLite nesta fase, validação no Rust, i18n tipado sem biblioteca.

### 2.4 PR 3 — Dock e painéis

1. **Dock na direção escolhida no mock**: ícones em quadrado arredondado, **rótulo flutuante** com o nome ao passar o mouse, **ponto indicador** (nesta fase marca o painel aberto), **separador** entre grupos e **ampliação** se V1 for viável.
2. **Itens do dock**: apenas **Configurações** e **Sobre**. Nenhum item de demonstração.
3. **Painel Configurações**: Tema (Sistema/Claro/Escuro), Posição (Embaixo/Em cima), Tamanho (Pequeno/Médio/Grande) e Opacidade (Translúcido/Sólido), todos aplicados na hora.
4. **Painel Sobre**: versão, licença MIT e link para o repositório.
5. **Bandeja completa**: Mostrar/ocultar dock, Configurações (abre o painel) e Sair.
6. **Dock em cima**: layout e ancoragem do painel funcionando nas duas bordas.
7. **Acessibilidade mínima**: itens navegáveis por teclado (Tab e setas) com foco visível, `aria-label` em todo item só com ícone, contraste AA nos dois temas e nas duas opacidades, animações desligadas com "reduzir movimento" do Windows.
8. Roadmap: itens e critérios da Fase 1 marcados conforme verificados.

## 3. O que esta spec deliberadamente NÃO entrega

- Dock nas bordas laterais (vertical).
- Ocultação automática e reserva de espaço na tela (AppBar): Fase 4.
- Múltiplos monitores e comportamento em tela cheia: Fase 4.
- Inicialização com o Windows: Fase 4.
- Atalho global de teclado para mostrar/ocultar: backlog.
- Atalhos de apps, arquivos e prateleira: Fase 3. Widgets: Fase 2.
- Animação de "pulo" ao abrir algo: Fase 3.
- Escolha da cor de destaque pelo usuário (os tokens já nascem preparados).
- Interface em inglês (a estrutura nasce pronta para ela).
- Painel redimensionável; deslocamento do dock ao longo da borda.
- Testes end-to-end.
- Teste com leitor de tela.

## 4. O que esta spec deliberadamente NÃO decide (ainda)

- **Valor exato da CSP**: definido na implementação do PR 1, dentro das restrições da §2.1 e do teste-guarda do PR 2.
- **Paleta, destaque, tipografia e valores exatos dos presets de tamanho**: saem do mock.
- **Ampliação, vidro real e Alt+Tab**: dependem de V1–V3; as alternativas já estão decididas.
- **Ferramenta de E2E**: decidir na Fase 2, quando houver o fluxo "criar nota, reiniciar, verificar".
- **Biblioteca de i18n**: só se plurais ou formatação complexa aparecerem, com a chegada do inglês.
- **Limite de memória aceitável**: esta fase só mede a linha de base.

## Riscos

| Risco | Impacto | Mitigação |
|---|---|---|
| Click-through dinâmico (V1) instável ou custoso em CPU | Médio | Validação isolada no PR 1; alternativa sem ampliação já decidida. |
| Acrylic sem cantos grandes (V2) | Médio | Vidro simulado já decidido; opção Sólido nas configurações. |
| Janela sempre no topo atrapalhando apps em tela cheia | Médio | Registrar o comportamento observado; tratamento na Fase 4. |
| Foco: abrir o painel rouba o foco do app em uso | Baixo | Comportamento esperado para um popover; registrar no ADR 0006. |
| Duas WebViews dobrando o consumo de memória | Baixo | Medição no PR 1; janela de painel reaproveitada. |
| Mock e implementação divergirem | Médio | Tokens extraídos do mock aprovado são a fonte do CSS do PR 2. |

## Verificação

Cada PR mostra a saída dos comandos, não a presume. Em todos os PRs: CI verde e todos os checks locais da spec 0001 (frontend e Rust, com `--locked`).

**PR 1 — Janela**
1. `pnpm tauri dev` e `pnpm tauri build --no-bundle` funcionam com a CSP definida (sem erros de CSP no console da WebView).
2. Dock aparece sem borda, transparente, centralizado acima da barra de tarefas, sempre no topo, ausente da barra de tarefas. Alt+Tab: resultado de V3 registrado.
3. Mudar a escala do Windows (100/125/150%) e a resolução reposiciona o dock corretamente.
4. Painel: abre ancorado, fecha com Esc, com clique fora e com novo clique no item; nunca sai da área útil.
5. Bandeja: ocultar, mostrar e sair funcionam; Alt+F4 oculta.
6. V1 e V2: resultado registrado no ADR 0006, com a alternativa aplicada quando for o caso.
7. Memória em repouso medida e registrada nesta spec.
8. Confirmação visual do usuário no Windows real.

**Mock**
9. `docs/design/` contém o mock com B1 e B2 em todos os estados da §2.2; o usuário escolhe uma variante; tokens registrados.

**PR 2 — Fundação**
10. Testes-guarda de capability e CSP falham ao ampliar uma permissão ou relaxar a CSP (bite-check mostrado) e passam ao restaurar.
11. Testes Rust de configurações cobrem: padrão, validação de cada campo, arquivo ausente, corrompido e inválido, e gravação atômica.
12. Mudar uma configuração, encerrar e reabrir o app: valor preservado. Apagar ou corromper o `settings.json`: o app abre com o padrão e registra aviso no log.
13. O log não contém caminhos de usuário nem conteúdo de configurações (inspeção do arquivo).
14. Executar o app duas vezes: um único dock.
15. Remover uma chave de `pt-BR.ts` quebra o `typecheck` em quem a usa.
16. Teste de componente: painel que lança erro mostra estado de falha e o resto continua funcionando.

**PR 3 — Dock e painéis**
17. Critérios de aceitação da Fase 1 do roadmap: dock abre e fecha sem falhas; painéis abrem e fecham; tema e preferências sobrevivem a reinicializações; interface utilizável em 100/125/150%; falha de um painel não derruba o dock.
18. Teclado: Tab e setas percorrem os itens; Enter abre o painel; Esc fecha; foco sempre visível.
19. Contraste AA conferido nos dois temas e nas duas opacidades.
20. Com "reduzir movimento" ligado no Windows, nenhuma animação de ampliação ou transição longa.
21. Confirmação visual do usuário no Windows real.

## Decisões da sessão de grill

| # | Decisão |
|---|---|
| Q1 | Janelas separadas para dock e painel (área transparente não deixa o clique passar). |
| Q2 | Bordas inferior e superior; padrão embaixo, centralizado. |
| Q3 | Sempre no topo; sem ocultação automática e sem AppBar. |
| Q4 | Ícone na bandeja para mostrar, ocultar e sair. |
| Q5 | Itens reais: Configurações e Sobre; sem itens de demonstração. |
| Q6 | Tema Sistema/Claro/Escuro, padrão Sistema; escolha de destaque depois. |
| Q7 | Tamanho salvo como número, com presets por cima. |
| Q8 | `settings.json` próprio validado no Rust, dois comandos IPC. |
| Q9 | `tauri-plugin-log`, sem dados sensíveis. |
| Q10 | Direção visual proposta e aprovada em mock antes de codar. |
| Q11 | Só o monitor principal; reposiciona em mudanças de resolução/escala. |
| Q12 | Três PRs, começando pelo risco (janela). |
| Q13 | Testes-guarda de capability e CSP. |
| Q14 | Um painel por vez; fecha com novo clique, Esc e perda de foco (inclusive Configurações). |
| Q15 | Painel ancorado ao item, contido na área útil, tamanho fixo. |
| Q16 | Janela de painel única, criada oculta e reaproveitada. |
| Q17 | Formato do `settings.json` v1 (§2.3). |
| Q18 | Salva a cada mudança; Rust fonte da verdade; gravação atômica; evento `settings-changed`. |
| Q19 | Alt+F4 no dock oculta; só "Sair" encerra. |
| Q20 | Instância única. |
| Q21 | Sem atalho global nesta fase. |
| Q22 | Ordem: PR 1 → mock → PR 2 → PR 3; mock em `docs/design/`. |
| Q23 | Interface em PT-BR, estruturada para inglês. |
| Q24 | Acessibilidade mínima (teclado, rótulos, contraste AA, reduzir movimento). |
| Q25 | E2E adiado para a Fase 2. |
| Q26 | Módulo Rust domain/persistence/commands, sem trait de armazenamento, `thiserror`. |
| Q27 | i18n com dicionário tipado próprio e `useT()`; `i18n.rs` para a bandeja. |
| Q28 | Escopo do PR 1 (§2.1). |
| Q29 | Direção B (dock no estilo macOS) com identidade própria; inspiração em docks Linux. |
| Q30 | Ampliação desejável, validada no PR 1 (V1). |
| Q31 | Se não houver desfoque real com cantos grandes, vidro simulado (V2). |
| Q32 | Rótulo flutuante, ponto indicador e separador; "pulo" na Fase 3. |
| Q33 | Mock com B1 (Big Sur) e B2 (Prateleira); identidade do DockMax em tokens, sem ligação com a marca do autor. |
| Q34 | Opacidade Translúcido/Sólido nas configurações. |
