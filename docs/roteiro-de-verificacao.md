# Roteiro de verificação depois da fase D

A rodada única de verificações que o autor decidiu fazer depois da fase D (roadmap, "Verificações pendentes", 2026-10-26). Reúne tudo o que as specs e os PRs marcaram como **não verificado**. Cada passo diz o que fazer e o que deve acontecer.

Use o build de release mais novo, com o mouse e o teclado de verdade. Ao fechar o DockMax, confira se a barra do Windows voltou como estava. Quando algo falhar, anote o passo, o que aconteceu e, se der, uma captura.

**Legenda:** `[x]` confirmado (com a data e quem confirmou) · `[ ]` a fazer.

## Já confirmado pelo autor (2026-10-26)

- [x] Arrastar para reordenar no dock nativo, com o mouse de verdade: o ícone sobe, os outros abrem espaço, e ao soltar ele desliza até o lugar ([PR #28](https://github.com/maxeinstein-dev/DockMax/pull/28)).
- [x] Arrastar widgets e apps abertos, e misturar apps e widgets pelo arrasto ([PR #26](https://github.com/maxeinstein-dev/DockMax/pull/26)).
- [x] Arrastar um ícone do painel da bandeja para o dock ([PR #26](https://github.com/maxeinstein-dev/DockMax/pull/26)).
- [x] O botão direito num ícone da bandeja no dock abre o menu do programa em cima do ícone ([PR #26](https://github.com/maxeinstein-dev/DockMax/pull/26)).
- [x] Um dock em cada monitor, nos dois monitores do autor ([PR #27](https://github.com/maxeinstein-dev/DockMax/pull/27)).
- [x] Ocultar automaticamente: desce, sobe na borda e fica à mostra com o ponteiro em cima ([PR #29](https://github.com/maxeinstein-dev/DockMax/pull/29)).
- [x] Abrir o DockMax pelo menu Iniciar traz o dock de volta.

## Resultado da rodada de 2026-10-27

O autor rodou os blocos 1 a 4 com o build do main depois do #29.

- **Passou:** tela cheia no segundo monitor com o principal em uso: só o dock do segundo some.
- **Falhou:**
  - Um programa fixado, quando aberto, aparece duas vezes: no grupo dos fixados e no dos abertos.
  - O ícone do próprio DockMax não aparece na bandeja.
  - O menu de um ícone da bandeja clicado no dock do segundo monitor abre no monitor principal. Causa: o FortiClient e outros programas "system aware" enxergam a tela na escala do monitor principal, e a posição do ícone ia em pixels físicos (o segundo monitor está em 125 %).
  - Os painéis do Windows (Wi-Fi, volume e bateria) abrem sempre no monitor principal. O Windows só os abre pelo atalho Win+A, sem escolha de monitor.
  - Com tela cheia no monitor principal e o segundo monitor em uso, o dock do principal volta por cima do vídeo.
- **Não feito:** o instalador (não há outra máquina) e o uso longo.
- **Retirado:** "Ícones dos apps". É um seletor de pacotes lidos de uma pasta, e o DockMax não traz pacote embutido, então só há "Original" (ver limitações).

## 1. Cliques no dock (uns 10 minutos)

- [ ] **Esquerdo num programa fixado fechado:** abre o programa, com a animação de abertura escolhida.
- [ ] **Esquerdo num programa aberto que não está na frente:** traz a janela para a frente.
- [ ] **Esquerdo de novo, com ele na frente:** minimiza.
- [ ] **Esquerdo num programa com várias janelas:** passa para a próxima janela a cada clique.
- [ ] **Meio:** abre uma nova instância (ou nada, se "Botão do meio" estiver em "Nada").
- [ ] **Direito num programa fixado:** abre o menu com a Jump List, "Fechar janela" e "Remover do dock".
- [ ] **Direito num programa aberto não fixado:** abre o menu com nova instância, fechar janela e "Fixar no dock".
- [ ] **Fechar janela pelo menu**, num programa com algo não salvo (o Bloco de Notas com texto novo, por exemplo): o programa pergunta se quer salvar, e nada se perde.
- [ ] **Direito no espaço vazio do dock:** o menu tem Gerenciador de Tarefas, Configurações, Ocultar automaticamente (com a marca) e Sair, e cada um faz o que diz.
- [ ] **Menu do ícone do DockMax na área de notificação:** os mesmos itens, e a marca de "Ocultar automaticamente" acompanha a configuração.

## 2. Aparência do dock nativo (uns 15 minutos)

- [ ] Os três efeitos ao passar o mouse (Ampliação, Escala, Nenhum), com o mouse passando devagar e depressa.
- [ ] Os nove estilos, um por um, e os reflexos onde o estilo tem.
- [ ] O dock na **borda de cima**: posição, faixa reservada, menus abrindo para baixo, arrasto, ocultar automaticamente.
- [ ] **Escala de 150 %** num monitor: o dock nítido, no tamanho certo e com a faixa certa.
- [ ] As opções novas de Configurações (Dock em todos os monitores, Ocultar automaticamente e os dois tempos), vistas no app de Configurações de verdade.

## 3. Arquivos e janelas (uns 10 minutos)

- [ ] **Soltar no dock**, vindo do Explorador de Arquivos, um `.exe` e um atalho `.lnk`: os dois são fixados. Um `.txt` é ignorado.
- [ ] **Áreas de trabalho virtuais do Windows** (Win+Tab › Nova área de trabalho): o dock aparece nas duas, e os programas abertos de cada área se comportam como esperado.

## 4. Dois monitores (uns 10 minutos)

- [ ] O mouse no **segundo dock**: abrir um painel (abre no monitor dele), um menu e um arrasto.
- [ ] **Ligar e desligar um monitor** com o DockMax aberto: o dock dele some e volta, e a faixa é devolvida.
- [ ] Desligar e religar "Dock em todos os monitores" com o DockMax aberto.
- [x] **Tela cheia de verdade no segundo monitor** (um vídeo do YouTube em tela cheia): só o dock daquele monitor some, e usar o outro monitor não o traz de volta por cima do vídeo. (Autor, 2026-10-27.)
- [ ] **Tela cheia no monitor principal**, idem, e um jogo em tela cheia exclusiva, se houver.

## 5. Instalador (uns 20 minutos, numa máquina limpa ou virtual)

- [ ] Instalar numa máquina limpa ou numa máquina virtual com Windows 11.
- [ ] Instalar no **Windows 10**: o que funciona e o que não.
- [ ] Atualizar por cima de uma versão anterior **com o DockMax aberto**.
- [ ] Desinstalar: o programa sai, os dados ficam (a pasta em `%APPDATA%`), e a barra do Windows volta.

## 6. Longo prazo

- [ ] **Uso longo (algumas horas)**: memória privada e CPU em repouso no começo e no fim (o Gerenciador de Tarefas, aba Detalhes, coluna "Memória (conjunto de trabalho privado)").
- [ ] **Leitor de tela (Narrador, Ctrl+Win+Enter):** o dock é lido como barra de ferramentas, cada item pelo nome, e "Invocar" age como o clique. Os painéis e o app de Configurações são navegáveis pelo teclado.

## Depois da rodada

Cada passo que falhar vira um item com o defeito descrito; os que passarem ficam marcados aqui, com a data. O roadmap aponta para este arquivo.
