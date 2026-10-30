# Spec 0005 — Fase A: visual do dock

- **Status:** A1 entregue (#9). Do A2, o tamanho de ícone contínuo (32–96) entrou e as bordas esquerda e direita (dock vertical) não: o dock vai só embaixo ou em cima (`DockEdge`). O A3 virou o app de Configurações ([spec 0011](0011-arquitetura-hibrida.md) H1, [spec 0014](0014-rebranding-configuracoes-e-menus.md)). O Canarinho saiu em 2026-10-30 (#82).
- **Data:** 2026-10-22
- **Origem:** [spec 0004](0004-paridade-primodock.md) (fases A–G) e [ADR 0009](../adr/0009-dock-substitui-a-barra-de-tarefas.md)

## 1. O problema, medido

O dock de hoje tem um visual só (barra "Big Sur"), uma ampliação fixa (1,4× no item sob o mouse e 1,15× nos vizinhos), um tamanho em três presets e um ponto de painel aberto sempre em `--accent`. As configurações (`settings.json` v1) guardam tema claro/escuro, borda (embaixo ou em cima), tamanho e opacidade.

O PrimoDock oferece 9 temas de barra com personalização por tema, três efeitos de passar o mouse, dois indicadores com paleta de cores, ícones de qualquer tamanho e as quatro bordas.

## 2. Recortes

A fase é entregue em três recortes, cada um um PR.

| Recorte | Entrega | Toca em |
|---|---|---|
| **A1** | 9 temas de barra e personalização (borda, cantos, reflexos, restaurar), efeito ao passar o mouse (Nenhum, Escala, Ampliação) com escala, indicador (Ponto, Traço) e cor | `settings` (Rust e TS), `layout.ts`, `Dock.css`, `tokens.css`, painel Configurações |
| **A2** | Geometria: tamanho de ícone contínuo (32–96, padrão 52), espaço até a borda, bordas esquerda e direita (dock vertical) | `windows/dock.rs`, `windows/panel.rs` (âncora do painel), `layout.ts`, CSS |
| **A3** | Painel Aparência com cartões de miniatura, no formato da janela de configurações do PrimoDock | Janela de configurações nova |

Fora da Fase A: formato dos ícones (Placas arredondadas, Mesmo tamanho, Como vieram) e animação de abertura (Pulo, Pulso, Tremor) só fazem sentido com apps no dock, então entram com a Fase B. Temas sazonais ficam para a Fase G.

## 3. A1 em detalhe

### 3.1 Configurações

O `settings.json` continua na versão 1: os campos novos são aditivos, com padrão quando faltam (as regras do ADR 0007 não mudam).

Dentro de `dock`:

| Campo | Valores | Padrão |
|---|---|---|
| `style` | `native`, `liquidGlass`, `midnight`, `rocket`, `aquaShelf`, `neon`, `canary`, `paper`, `floating` | `native` |
| `tweaks` | mapa de estilo para `{ border?: bool, corners?: 0–100, reflections?: bool }` | vazio |
| `hover` | `none`, `scale`, `magnify` | `scale` |
| `hoverScale` | porcentagem, 100–160 | 110 |
| `indicator` | `dot`, `dash` | `dot` |
| `indicatorColor` | `null` (padrão do tema) ou `#RRGGBB` | `null` |

Regras de validação no Rust: `corners` de 0 a 100, `hoverScale` de 100 a 160, cor no formato `#RRGGBB` com 6 dígitos hexadecimais. Um valor inválido é recusado com um código novo (`dockStyleInvalid`) sem expor o valor. A leitura do arquivo segue o que já existe: campo ausente vira o padrão, campo desconhecido é ignorado.

O `opacity` existente (translúcido ou sólido) continua valendo para os temas que têm superfície (Nativo e Meia-noite); os outros temas têm as próprias cores.

### 3.2 Temas

Cada tema é um bloco de CSS sobre `.dock-bar`, escolhido por `data-style` na raiz e alimentado por tokens (`--bar-bg`, `--bar-border`, `--bar-glow`, `--bar-radius`). O valor da personalização chega por `data-border` e `data-reflect` (resolvidos na TS com o padrão de cada tema) e por `--corners` (0 a 1).

| Tema | Barra | Borda | Brilho | Cantos | Reflexos |
|---|---|---|---|---|---|
| Nativo | superfície translúcida atual | fina | não | 40 % | não |
| Liquid Glass | vidro claro (degradê branco 18 % para 6 %), realce interno | branca 1,5 px | leve | 100 % | não |
| Meia-noite | quase preta (92 %) | branca fina | não | 20 % | não |
| Rocket | azul-marinho | azul 1 px | azul (8 px) | 20 % | não |
| Prateleira Aqua | sem barra; prateleira prateada em perspectiva sob os ícones | sem | não | n/a | sim |
| Neon | preta | degradê ciano para magenta | ciano e magenta | 60 % | não |
| Canarinho | degradê amarelo para verde, brilho no topo | escura fina | não | 100 % | não |
| Papel | branca opaca | sem | sombra leve | 30 % | não |
| Flutuante | sem barra | sem | não | n/a | sim |

Os reflexos usam `-webkit-box-reflect` com uma máscara que mostra só os primeiros pixels da imagem espelhada (no máximo o espaço `--pad` abaixo do ladrilho), para caber na janela.

O brilho respeita a folga transparente da janela (`ROOM`, 8 px): nenhum brilho passa dela, porque a janela corta. O dock continua a só capturar o mouse na barra (`bar` no layout enviado ao Rust).

### 3.3 Efeito ao passar o mouse

- **Nenhum:** nada cresce.
- **Escala:** só o item sob o mouse cresce `hoverScale` %, sem levantar e sem mexer nos vizinhos.
- **Ampliação:** o comportamento atual (1,4× e 1,15×, com elevação).

`layout.ts` calcula tudo a partir do modo, para o retângulo de captura e o tamanho da janela continuarem iguais ao que o CSS desenha. Com "reduzir movimento" do Windows nada cresce, como hoje.

### 3.4 Indicador

O ponto de "painel aberto" ganha o estilo Traço (14 × 3 px, cantos redondos) e uma cor. Sem cor escolhida, vale a cor do tema (branca nos temas escuros, grafite no Papel).

### 3.5 Painel Configurações

Sem janela nova (isso é o A3): o painel atual ganha um subpanel "Aparência do dock", aberto por uma linha de navegação, com o seletor do tema, as chaves Borda e Reflexos, o controle de Cantos, o efeito ao passar o mouse com a escala, o indicador e a cor, e "Restaurar" para o tema. Os controles novos seguem o `OptionGroup` e o `SelectField` que existem.

## 4. Verificação

1. Rust: o padrão e a leitura de arquivo v1 antigo (sem os campos novos); cada campo fora da faixa é recusado; um `settings.json` com `style` desconhecido cai no padrão sem derrubar a inicialização.
2. TS: a resolução do padrão de cada tema com e sem personalização; `layout.ts` para cada modo (retângulos e tamanho de janela); `Dock.tsx` aplica `data-style`, `data-border` e `data-reflect`.
3. Estilo: um teste lista os nove temas e exige um bloco de CSS para cada, com os tokens definidos (para um tema novo não ficar sem visual).
4. No app real (`pnpm tauri build --no-bundle` e captura via CDP): cada tema em claro e escuro, cada efeito, os dois indicadores.
5. Memória em repouso (processo e WebView2) comparada com a linha de base de 182 MB; o orçamento de 220 MB vale.

## 5. Riscos

- O brilho de Rocket e Neon pode parecer cortado perto da borda da janela; a mitigação é limitá-lo a `ROOM`. Se não ficar bom, A2 aumenta a folga da janela.
- A prateleira em perspectiva (Aqua) é desenhada em CSS (`clip-path`); se ficar fraca, vira um SVG próprio.
- Os ladrilhos continuam no estilo "Big Sur" do `DESIGN.md` até a Fase B (pacotes de ícones); os temas valem para a barra, não para o ladrilho.

## 6. Resultado da verificação do A1 (2026-10-22)

- **Testes:** 915 do frontend e 367 do Rust passam; clippy, lint, typecheck e `prettier --check` limpos.
- **App real** (`pnpm tauri build --no-bundle`, executado com cada tema sobre as configurações do autor, restauradas ao fim): Nativo (escuro e claro), Liquid Glass, Meia-noite, Rocket, Prateleira Aqua, Neon, Canarinho, Papel e Flutuante desenham como na spec. O degradê do Neon, o brilho e a pílula do Canarinho saem bem; o brilho do Rocket fica contido (a folga de 8 px da janela corta o resto).
- **Reflexos:** a primeira versão não aparecia, porque a máscara do `-webkit-box-reflect` vale para a imagem antes de espelhar (o degradê estava invertido). Corrigido; no Flutuante e na Prateleira Aqua o reflexo aparece, discreto, limitado aos 7 px de folga da barra.
- **Memória em repouso:** 179,2 MB privados (dockmax e WebView2 filhos, 31 s depois de abrir), contra 182 MB da linha de base e o orçamento de 220 MB.
- **Não verificado no app real:** a borda em cima, o painel "Aparência do dock" na janela do painel (só em testes), o indicador (precisa de um painel aberto) e os temas em tela com escala diferente de 100 %.
- **Fraquezas conhecidas:** a prateleira Aqua é mais fina que a do PrimoDock; o brilho do Rocket e do Neon é cortado pela folga da janela, e o A2 pode aumentá-la.
