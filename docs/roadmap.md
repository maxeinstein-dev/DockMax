# Roadmap — DockMax

**Produto:** DockMax  
**Plataforma inicial:** Windows 11  
**Objetivo:** ferramenta de uso pessoal que também demonstre competências de desenvolvimento desktop e arquitetura.  
**Status:** proposta inicial; estimativas são indicativas. **Atualização de 2026-10-26:** o produto passou a ser um dock que substitui a barra de tarefas (ADR 0009) e o plano das fases 3 e 4 abaixo foi superado pelas fases A a G da [spec 0004](specs/0004-paridade-primodock.md). O que define o MVP está em "Escopo do MVP"; o andamento das fases A a G, em "Andamento das fases da spec 0004".

## Escopo do MVP (decisão do autor, 2026-10-26)

O MVP é o que já existe, com os **ícones que os programas já têm**:

- o dock no lugar da barra de tarefas do Windows, com restauração garantida (ADR 0009);
- a fase A (visual): nove estilos, efeito ao passar o mouse, indicador, posições;
- a fase B (apps): programas fixados com o ícone real, fixar e desafixar, reordenar arrastando, soltar um programa ou atalho no dock, e o menu do programa (a Jump List do Windows);
- relógio com data, bandeja do sistema hospedada pelo DockMax e os widgets Relógio, Progresso do tempo e Nota.

Ficam **para o futuro**, fora do MVP: a personalização de ícones (ícone por app, imagem própria, chaves semânticas e um pacote embutido, recortes B4b e B4c da [spec 0007](specs/0007-fase-b4-icones.md); a leitura de pacotes de ícones, B4a, já está pronta e fica como recurso avançado), os apps abertos e as prévias de janela (fase D e E), os espaços de trabalho (fase C), os demais widgets (fase F) e os extras (fase G). A fase 5 abaixo é o que falta para fechar o MVP.

## Andamento das fases da spec 0004

Ordem decidida pelo autor em 2026-10-26, depois da fase D: **C → E → F → G**, cada uma com spec própria antes do código.

| Fase | Situação |
|---|---|
| **A. Visual** | Concluída: nove estilos, efeito ao passar o mouse, indicador, posições ([spec 0005](specs/0005-fase-a-visual.md)). |
| **B. Itens e ícones** | Concluída no escopo do MVP ([spec 0006](specs/0006-fase-b-apps.md), [spec 0007](specs/0007-fase-b4-icones.md)); a personalização de ícones (B4b, B4c) fica para o futuro. |
| **D. Windows** | **Concluída em 2026-10-26**: barra de aplicativos e barra do Windows oculta (ADR 0009), apps abertos, ativar e minimizar, menus ([spec 0009](specs/0009-fase-d-janelas.md)), tela cheia e um dock por monitor ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md)), dock nativo ([spec 0011](specs/0011-arquitetura-hibrida.md)) e ocultar automaticamente. Falta a [rodada de verificações](roteiro-de-verificacao.md). |
| **C. Espaços de trabalho** | **Concluída** ([spec 0013](specs/0013-fase-c-espacos-de-trabalho.md)): C1 (espaços, troca no dock e edição em Configurações), C2 (janelas por espaço, guardadas entre execuções) e C3 (mover janelas para outro espaço, troca em todos os monitores, indicador na cor do espaço, widgets em vários espaços, bolinhas no leitor de tela, troca animada). |
| **E. Prévias e bandeja** | **Concluída** ([spec 0015](specs/0015-fase-e-previas.md)): a bandeja (painel da seta, ícones no dock, menus no lugar), E1 (o cartão com as miniaturas ao vivo, ativar e fechar) e E2 (o cartão no leitor de tela e a entrada animada). |
| **F. Widgets** | **Em andamento** ([spec 0017](specs/0017-fase-f-widgets.md)): F1 (tiles largos, Atividade, Atividade de rede e a aba Widgets com o catálogo); F2 a F5 a fazer. |
| **G. Extras** | Depois da F. |

Fora das fases: o **lançador** no estilo do Ulauncher ([spec 0012](specs/0012-lancador.md)). O L1 entrou em 2026-10-27 (Alt+Espaço, busca nos apps, recentes), no [PR #33](https://github.com/maxeinstein-dev/DockMax/pull/33). O L2 (calculadora, busca na web por palavra-chave, Configurações do DockMax e do Windows) entrou no [PR #37](https://github.com/maxeinstein-dev/DockMax/pull/37). O L3 (personalizar o atalho) vem no PR seguinte. O L4 busca arquivos e pastas (o Everything, quando roda, e o índice do Windows Search); depois vêm o L5 (caminhos), o L6 (menu de ações por resultado) e o L7 (janelas abertas).

Também fora das fases: os **selos de notificação** ([spec 0016](specs/0016-selos-de-notificacao.md)), o número de não lidas que o app põe no botão da barra do Windows, lido pela UI Automation e desenhado no ícone do dock. Para isso a barra passa a ficar transparente e sem receber cliques, em vez de escondida.

## Visão do produto

DockMax será um dock personalizável para Windows, inspirado em aplicações de dock e widgets, mas com identidade própria. A interface principal reunirá atalhos e painéis de widgets. A integração com a barra de tarefas será evolutiva: o DockMax começará convivendo com a barra nativa e poderá incorporar capacidades de gerenciamento de janelas ao longo do tempo.

## Princípios de escopo

- Primeiro entregar um aplicativo local útil; não criar backend remoto sem necessidade.
- Separar widgets visuais de serviços nativos e integrações externas.
- Implementar widgets por prioridade e dependência técnica.
- Tratar recursos que dependem de hardware ou APIs externas como capacidades opcionais.
- Medir memória e CPU em repouso em todo PR que mude a interface e comparar com a linha de base. Orçamento até o MVP: memória privada em repouso (processo + WebView2) no máximo 20% acima da linha de base da Fase 1 (182 MB), ou seja, cerca de 220 MB. **Revisto em 2026-10-26 ([ADR 0010](adr/0010-gpu-no-processo-do-navegador.md)):** cerca de 180 MB com um dock e cerca de 15 MB por monitor a mais.
- Comercialização, suporte a macOS e sincronização em nuvem estão fora do escopo inicial.

## Fases

### Fase 0 — Preparação e validação técnica

**Estimativa:** 2–4 dias.

- [x] Criar repositório e configurar Git.
- [x] Instalar/verificar Node.js, Rust toolchain e dependências necessárias do Tauri no Windows.
- [x] Criar esqueleto Tauri 2 + React + TypeScript.
- [x] Confirmar build e execução local.
- [x] Configurar formatação, lint e testes.
- [x] Registrar versões e comandos de desenvolvimento no README.

**Critério de saída:** aplicação vazia abre localmente e pode ser compilada sem erros.

### Fase 1 — Casca do aplicativo e dock visual

**Estimativa:** 1–2 semanas.

- [x] Implementar janela do dock e comportamento inicial.
- [x] Criar componentes visuais do dock, ícones e painel.
- [x] Definir tokens de tema com variáveis CSS.
- [x] Permitir abrir/fechar painéis.
- [x] Implementar configurações básicas de posição, tamanho e tema.
- [x] Definir tratamento de erros e logs sem informações sensíveis.

**Critérios de aceitação**
- Dock abre e fecha sem falhas.
- Painéis podem ser abertos e fechados.
- Tema e preferências sobrevivem a reinicializações.
- Interface continua utilizável em uma escala de tela comum.
- Falhas de um painel não derrubam a janela principal.

Critérios verificados em 2026-10-15 (spec 0002, verificações 1–21, e teste manual do autor no build de release).

### Fase 2 — Núcleo de widgets e persistência

**Estimativa:** 1–2 semanas.

- [x] Definir o contrato de widget e o registro central.
- [x] Implementar persistência local e migrações.
- [x] Criar widgets Relógio, Progresso do tempo e Nota.
- [x] Criar estrutura inicial para Atalho e Arquivos.
- [x] Implementar validação de configurações.
- [x] Adicionar testes unitários para regras de negócio.

**Critérios de aceitação**
- Instâncias de widgets têm IDs estáveis.
- Configurações inválidas são tratadas sem bloquear a inicialização.
- Notas persistem após reiniciar o aplicativo.
- A lógica de negócio pode ser testada sem renderizar a interface.

Revisão de 2026-10-19 (fim da Fase 2): a direção foi mantida e o roadmap ganhou orçamento de memória, a divisão da Fase 3 por valor e risco, o spike da janela única na Fase 4 e a poda dos widgets de baixo valor. Detalhes nas specs das fases.

### Fase 3 — Atalhos, arquivos e prateleira

**Estimativa:** 1–2 semanas. Dividida em três partes, por valor no uso diário e por risco; cada parte é um PR.

**3a — Atalhos** (maior valor, menor risco)
- [ ] Esconder o dock sobre aplicativos em tela cheia (adiantado da Fase 4: é o que mais fere "nunca atrapalhar" no dia a dia, e a detecção da janela em primeiro plano já existe no código nativo).
- [ ] Abrir aplicativos, arquivos, pastas e URLs com validação (`opener` com caminhos e URLs conferidos no Rust).
- [ ] Permitir fixar, remover e reordenar atalhos.
- [ ] Restringir comandos nativos ao necessário.
- [ ] Link do repositório no painel Sobre abrindo o navegador (hoje é texto selecionável).

**3b — Arquivos**
- [ ] Implementar widget Arquivos para locais escolhidos pelo usuário.
- [ ] Definir o que ocorre quando um arquivo de referência é movido ou removido.

**3c — Prateleira** (só depois da validação técnica)
- [ ] Validação técnica: soltar arquivos (drag-and-drop) numa janela cujo click-through é alternado pelo Rust (ADR 0006). Alternativa já decidida antes de começar, como na Fase 1.
- [ ] Implementar Prateleira com drag-and-drop e referências a arquivos.

**Critérios de aceitação**
- Atalhos válidos abrem o destino esperado.
- Erros de caminho são apresentados de forma compreensível.
- A Prateleira não duplica arquivos automaticamente.
- Nenhum comando arbitrário pode ser executado diretamente a partir de conteúdo não confiável.

### Fase 4 — Integração com o Windows

**Estimativa:** 2–4 semanas; pode variar significativamente.

- [ ] Spike medido da janela única: dock e painel na mesma janela transparente, usando o click-through dinâmico já provado (ADR 0006, V1), para cortar um renderizador do WebView2. Decidir por ADR com a medição de memória, foco e blur.
- [ ] Orçamento de CPU em repouso, medido junto com a memória.
- [ ] Implementar provedores de bateria e rede.
- [ ] Investigar enumeração, foco, ativação e minimização de janelas.
- [x] Definir comportamento em múltiplos monitores: um dock por monitor, cada um com a sua faixa e a sua regra de tela cheia ([spec 0010](specs/0010-fase-d4-monitores-e-tela-cheia.md), D4b).
- [ ] Testar inicialização com o Windows.
- [ ] Avaliar limites de acesso a métricas de hardware.
- [ ] Adicionar testes e fallback quando uma API não estiver disponível.

**Critérios de aceitação**
- Dados indisponíveis são identificados como indisponíveis, não inventados.
- Erros de integração não encerram o aplicativo.
- O DockMax pode ser usado junto da barra de tarefas nativa.
- Comportamentos de foco e janela estão documentados.

### Fase 5 — Empacotamento e MVP

**Estimativa:** 1–2 semanas.

- [x] Gerar instalador Windows: NSIS, por usuário, em português, 2,6 MiB, não assinado ([spec 0008](specs/0008-fechamento-do-mvp.md)).
- [x] Testar instalação e desinstalação: feito em silêncio na máquina do autor (ver "Verificação do instalador"); **não** foi feito numa máquina limpa nem numa máquina virtual.
- [x] Verificar persistência de configurações e do banco após atualização manual, com uma migração real (v1 → v2) e o backup antes dela: coberto pelos testes do banco (migração com `dockmax.db.bak-v<n>`, falha de migração que deixa o arquivo como estava, banco danificado renomeado) e pela migração 1 → 2 (B1) já usada nos dados reais do autor.
- [x] Política de retenção dos arquivos `bak-v*` e `.corrompido-*` na pasta de dados: **nenhum é apagado pelo programa** (o DockMax nunca apaga dados do usuário; cada `bak-v*` vem de uma migração e é pequeno, e um `.corrompido-*` só existe se o banco foi danificado). A pasta e o motivo estão no README e em [limitações](limitacoes.md); apagar é decisão de quem usa.
- [x] Documentar limitações conhecidas (inclusive: leitor de tela nunca testado): [limitações](limitacoes.md).
- [x] README com "onde ficam seus dados" (`%APPDATA%\dev.maxsueleinstein.dockmax`). (feito na v0.2.0)
- [ ] Criar README com imagens, arquitetura, funcionalidades e instruções.
- [ ] ~~Criar uma demonstração curta do aplicativo.~~ Fora por agora (decisão do autor, 2026-10-26).
- [x] Verificar licença de dependências e atribuições necessárias: [licenças de terceiros](licencas-de-terceiros.md) (nenhuma exige abrir o código; os avisos para distribuir a outras pessoas ainda não foram gerados).

**Critérios de aceitação do MVP**
- [x] Instala e inicia em uma máquina Windows compatível (na do autor; ver as limitações).
- [x] Dock e painéis funcionam sem falhas críticas conhecidas.
- [x] Atalhos e preferências persistem.
- [x] Notas são salvas e recuperadas.
- [x] Temas são aplicados de forma consistente.
- [x] Memória privada em repouso dentro do orçamento (cerca de 220 MB) e CPU em repouso abaixo de 1%: ver "Medição de recursos".
- [x] Testes essenciais passam.
- [x] Documentação explica como executar, testar e compilar.
- [x] Limitações de gerenciamento de janelas estão explícitas.

### Verificações pendentes (decisão do autor, 2026-10-26: fazer todas depois da fase D)

A fase D acabou: a rodada única está no **[roteiro de verificação](roteiro-de-verificacao.md)**, passo a passo e em blocos. O que o autor já confirmou nos PRs da fase D (arrastar, dois monitores, ocultar automaticamente) está marcado lá. Resumo do que falta:

- [ ] Instalar e desinstalar numa máquina limpa ou virtual; Windows 10; atualizar por cima de uma versão anterior com o programa aberto.
- [ ] Os cliques com o **mouse físico** no dock: esquerdo (ativar, minimizar, próxima janela), meio (nova instância) e direito (menus), nos programas fixados e nos abertos; os menus do dock e da bandeja.
- [ ] Soltar um arquivo no dock vindo do Explorador de Arquivos (o teste usou um arraste simulado).
- [ ] A seção "Ícones dos apps" e as chaves novas de Configurações vistas no app de Configurações de verdade.
- [ ] Um programa que pergunta se quer salvar ao fechar pelo menu.
- [ ] Áreas de trabalho virtuais do Windows.
- [ ] Uso longo (horas): memória e CPU em repouso.
- [ ] Leitor de tela.
- [ ] O dock nativo (spec 0011 H3) com o **mouse de verdade**: os três efeitos ao passar o mouse; os oito estilos e os reflexos na tela; a borda de cima; escala de 150 %. (Arrastar para reordenar: confirmado pelo autor em 2026-10-26.)
- [ ] Um dock por monitor (spec 0010 D4b): o mouse no segundo dock, ligar e desligar um monitor com o DockMax aberto, mudar "Dock em todos os monitores" com ele aberto, tela cheia de verdade no segundo monitor. (Um dock em cada monitor: confirmado pelo autor em 2026-10-26.)
- [ ] O lançador (spec 0012 L4): ao vivo, buscar arquivos (misturados e com `f `), abrir um arquivo e uma pasta, os ícones de tipo, a roda do mouse, o Narrador lendo "Arquivo" e "Pasta", o Everything de verdade (também rodando como administrador) e a memória com o lançador aberto buscando arquivos.
- [ ] O lançador (spec 0012 L3): capturar um atalho novo em Configurações com o teclado físico (Alt+Espaço e combinações com Win dentro do WebView2) e com o leitor de tela.
- [ ] O lançador (spec 0012 L1): o Alt+Espaço com o teclado físico, o Narrador lendo a lista e a seleção, o IME de outro idioma na caixa de busca, o alto contraste, escala de 150 % e dois monitores.
- [ ] O lançador (spec 0012 L2): o Narrador lendo "Resultado copiado"; o editor de palavras-chave em Configurações com o teclado e o leitor de tela. (O Enter na conta, em `g dockmax` e em `wifi` o autor verificou em 2026-10-27.)

### Medição de recursos (build de release 0.3.0, 2026-10-26)

Máquina do autor (Windows 11, 16 processadores lógicos, um monitor, escala de 125 %), dock com a barra do Windows oculta, sem programas fixados além dos do autor, painel fechado, medido 25 s depois de abrir.

| Medida | Resultado | Meta |
|---|---|---|
| Memória privada em repouso (processo e os 6 processos do WebView2) | 174 MB aos 25 s; 187 MB 30 s depois | cerca de 220 MB |
| CPU em repouso (30 s) | 0,02 % do total dos 16 processadores | abaixo de 1 % |

A memória subiu 13 MB entre as duas medições; as duas ficam abaixo da meta. Não foi medido um uso longo (horas).

**Com o dock nativo** (spec 0011 H3, release, 2026-10-26, mesma máquina): **64,5 MB** de memória privada em repouso, num processo só (nenhum WebView2), 0 % de CPU; com um painel aberto o WebView2 volta enquanto ele estiver aberto, e sai 45 s depois de fechado (spec 0011 §7).

**Com um dock por monitor** (spec 0010 D4b, release, 2026-10-26, dois monitores): cerca de **88 MB** de memória privada com os dois docks, num processo só, medido 7 s depois de abrir.

### Verificação do instalador (2026-10-26)

Instalador `DockMax_0.3.0_x64-setup.exe`, instalado e desinstalado em silêncio (`/S`) numa pasta do usuário, na máquina do autor:

- instala sem administrador (`dockmax.exe` e `uninstall.exe`) e registra "DockMax 0.3.0" em Aplicativos instalados; **não** abre o programa sozinho;
- com a entrada de inicialização presente, o desinstalador a remove, remove os atalhos do Menu Iniciar e da Área de Trabalho, a pasta de instalação e a cópia do vigia (`dockmax-guard.exe`); os dados do usuário ficam intactos;
- o desinstalador deixou os atalhos quando a instalação foi feita num caminho em formato curto 8.3 (`C:\Users\MAXSUE~1\...`), que o Windows usa no `%TEMP%`; numa pasta de caminho normal não deixa nada. Quem instala na pasta padrão (`%LOCALAPPDATA%\DockMax`) não é afetado.

Não verificado: máquina limpa ou virtual, Windows 10, atualização por cima de uma versão anterior com o programa aberto.

## Backlog pós-MVP

### Produtividade
- [ ] Calendário
- [ ] Lembretes e notificações
- [ ] Área de transferência
- [ ] Melhorias na Prateleira e nos grupos de atalhos

### Sistema e mídia
- [ ] Atividade: CPU e memória; outros indicadores conforme viabilidade. Definir o escopo antes de qualquer PR.
- [ ] Música: sessão de mídia do Windows

### Só se sobrar (valor baixo para o custo; cada widget custou cerca de 2 mil linhas com testes na Fase 2)
- [ ] Ventoinhas e temperaturas: exige driver de sensores, com risco de segurança
- [ ] Conta-gotas de cores
- [ ] Beber água
- [ ] Uso do Claude: depende de dados que a conta pode não expor

### Rebranding de Configurações e menus

- [x] Configurações, os menus do dock e da bandeja e o resto das telas de apoio estão "muito simples" (autor, 2026-10-27). Referências: o acabamento do PrimoDock ([interface](reference/primodock/interface.md): abas com ícones coloridos em placa, menus com ícones e destaque), o tema WhiteSur GTK e o dock do ZorinOS. Spec: [0014](specs/0014-rebranding-configuracoes-e-menus.md), em quatro recortes, todos feitos em 2026-10-27: R1 (estrutura de Configurações e temas com miniatura), R2 (as outras escolhas em cartões), R3 (menus próprios no dock, na bandeja e no menu de um programa) e R4 (acabamento das seções e do painel da bandeja).

### Personalização do dock
- [ ] Cor de destaque escolhida pelo usuário (o token `--accent` já existe)
- [ ] Ponto indicador personalizável: cor, forma e visibilidade

### Integrações
- [ ] GitHub
- [ ] Clima

## Widgets planejados

1. Atalho
2. Arquivos
3. Prateleira
4. Relógio
5. Progresso do tempo
6. Calendário
7. Lembretes
8. Nota
9. Música
10. Bateria
11. Atividade
12. Ventoinhas
13. Rede
14. Conta-gotas
15. Área de transferência
16. Beber água
17. Uso do Claude
18. GitHub
19. Clima

**Nota:** a lista consolidada contém 19 itens. O widget “Atividade” deverá ter seu escopo definido antes da implementação (por exemplo, CPU/RAM ou atividade de aplicativos).

## Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| Substituição completa da barra de tarefas | Alto | Manter convivência no MVP; evoluir por capacidades |
| Memória do WebView2 (182 MB na Fase 1; no PR 2 da Fase 2, 202–264 MB antes e 170–231 MB depois de dividir um renderizador e baixar o alvo de memória em repouso, spec 0003 §2.12) | Alto | Orçamento por fase, medição em todo PR de interface, spike da janela única na Fase 4 |
| Drag-and-drop numa janela com click-through alternado pelo Rust | Médio | Validação técnica antes da Prateleira (3c), com alternativa decidida |
| APIs de hardware inconsistentes | Alto | Provedores substituíveis e estado “indisponível” |
| Métricas do Claude não acessíveis para conta individual | Alto | Identificar fonte e tipo de conta; não prometer limites não disponíveis |
| Consumo excessivo de recursos | Médio | Atualização centralizada e medição |
| Credenciais expostas | Alto | Armazenamento seguro e permissões mínimas |
| Escopo crescendo demais | Alto | Definir critérios de aceite por fase e adiar extras |

## Processo de trabalho com IA

Para cada tarefa:
1. Descrever objetivo e critérios de aceitação.
2. Pedir alternativas e riscos antes de codificar quando houver decisão arquitetural.
3. Implementar um incremento pequeno.
4. Executar testes e revisar o diff.
5. Atualizar documentação quando uma decisão mudar.
6. Criar commits atribuídos somente ao usuário, conforme a configuração de autoria desejada.
7. Classificar cada PR como caro ou barato no plano: PR barato tem um revisor e dispensa a segunda rodada se não houver achado bloqueante; usar menos lanes quando as peças são pequenas.
8. Antes do teste manual do usuário, capturar o build real via CDP nos dois temas (pega contraste e controle faltando antes do teste).
9. Medir memória e CPU em repouso em todo PR que mude a interface.
10. Versionar por fase (v0.2.0 no fim da Fase 2), para o painel Sobre e o instalador nascerem alinhados.
11. A `main` é protegida (2026-10-27): o PR só entra com a CI (`checks`) verde e a branch atualizada com a `main`, também para o dono; a branch é apagada no merge. Um PR empilhado sobre outro fica vermelho no job `base` até apontar para a `main`: mergeie primeiro o de baixo (o #45 caiu na branch do R1 por isso).

## Estimativa geral

Com dedicação de aproximadamente 10–15 horas por semana, um MVP básico pode levar cerca de 5–9 semanas. A parte de gerenciamento avançado de janelas pode estender o cronograma. As estimativas devem ser revisadas após a Fase 0.
