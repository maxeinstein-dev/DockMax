# Spec 0009 — Fase D: janelas e programas em execução

A [spec 0004](0004-paridade-primodock.md) coloca na fase D o que falta para o dock ocupar de fato o lugar da barra de tarefas: saber quais programas estão abertos, ativar e minimizar, um dock por monitor e ocultar automático. O dock já reserva a borda e esconde a barra do Windows ([ADR 0009](../adr/0009-dock-substitui-a-barra-de-tarefas.md)); hoje ele **não sabe** quais janelas existem ([limitações](../limitacoes.md)).

## 1. Recortes

| Recorte | Entrega |
|---|---|
| **D1** | Os programas **fixados** que estão abertos ganham o indicador; o clique neles ativa a janela (ou minimiza, ou alterna entre as janelas do programa) em vez de abrir outra instância |
| **D2** | Os programas abertos **não fixados** aparecem no dock depois de um separador ("Mostrar apps abertos"), com o mesmo clique e o menu |
| **D3** | Menu do botão direito de um programa aberto: "Fechar janela", "Fixar no dock" |
| **D4** | Um dock por monitor |
| **D5** | Ocultar automático do dock |

As prévias de janela são a fase E. Esta spec detalha o D1; os outros ganham seção quando começarem.

## 2. O spike (feito)

Perguntas: dá para saber, com pouco custo, quais janelas o Windows daria um botão na barra, e a qual programa fixado cada uma pertence?

- **Quais janelas:** `EnumWindows` filtrado como a barra faz: visível, sem `WS_EX_TOOLWINDOW` (a menos que `WS_EX_APPWINDOW`), sem dono (a menos que `WS_EX_APPWINDOW`), sem `WS_EX_NOACTIVATE`, não "encoberta" pelo DWM (`DWMWA_CLOAKED`: aplicativos da Loja suspensos e janelas de outra área de trabalho virtual) e com título. Foram excluídas as janelas do próprio DockMax.
- **A quem pertence:** o AppID que a janela declara (`SHGetPropertyStoreForWindow`, `System.AppUserModel.ID`); para um aplicativo empacotado (Loja), o AppID do pacote do processo (`GetApplicationUserModelId`); e o executável do processo (`QueryFullProcessImageNameW`).
- **Como casa com um programa fixado:** o id de um programa da pasta Aplicativos é o AppID (Loja) ou um nome de análise de atalho; o primeiro casa pelo AppID da janela, o segundo pelo **alvo do atalho** (`System.Link.TargetParsingPath`) igual ao executável da janela. Verificado nesta máquina com Chrome, Claude (Loja), Calculadora (Loja, janela dentro do `ApplicationFrameHost.exe`) e o Mapa de Caracteres (clássico, id `{GUID}\charmap.exe`): os quatro casaram, cada um por um dos dois caminhos.
- **Quando olhar:** não há laço. Um `SetWinEventHook` (fora de processo, sem injeção) avisa de criar, destruir, mostrar, esconder, encobrir, minimizar e trocar o primeiro plano; um temporizador de 200 ms que cada aviso reinicia junta uma rajada num só `EnumWindows`. Mudança de título **não** é ouvida (seria a mais frequente e o D1 não precisa dela). Um programa **encerrado à força** não avisa que as janelas sumiram (verificado: nenhum evento limpou a marca), então a cada 5 s o vigia pergunta `IsWindow` dos identificadores que guarda (sem COM, nada de enumerar) e só olha de novo se um sumiu.

## 3. D1 em detalhe

- **O que o Rust sabe:** por programa fixado, quantas janelas tem abertas e se uma delas está no primeiro plano. O Rust guarda também os identificadores das janelas (`HWND`); **a página nunca os vê nem os envia**: ela pede "ativar o programa fixado `<id>`".
- **Comandos** (só do dock, pinados no guarda): `list_open_apps` devolve `[{ id, count, active }]` (só os programas abertos); `activate_app(id)` ativa. O evento `open-windows-changed` leva a mesma lista a cada mudança.
- **O clique** num programa fixado: sem janelas, abre como antes (com a animação de abertura); com janelas, `activate_app`:
  1. se uma janela dele está no primeiro plano e é a única, **minimiza**;
  2. se está no primeiro plano e há outras, passa para a **próxima** (ordem estável das janelas);
  3. senão, ativa a mais acima, **restaurando-a** se estiver minimizada.
- **O indicador:** a marca do item (ponto ou traço, na cor escolhida em Aparência) fica visível enquanto o programa tem janela; no primeiro plano fica cheia, e nos demais, mais fraca.
- **Custo:** sem sondagem; cada rajada de avisos custa um `EnumWindows` e, para as janelas com botão, a leitura do AppID e do executável. A memória e a CPU em repouso são medidas no fim (orçamento: [roadmap](../roadmap.md)).

## 4. Testes

- Rust: a regra de casamento (AppID sem diferenciar maiúsculas; executável contra alvo; programa sem alvo; dois programas com o mesmo executável); o resumo por programa (contagem, ativo); a escolha de a quem passar o foco (minimizar, próxima, mais acima).
- Página: o indicador muda com o evento; o clique abre ou ativa conforme o estado.
- App real: abrir Bloco de notas e Calculadora fixados e ver o indicador; clicar ativa, de novo minimiza; fechar a janela apaga o indicador.

## 5. Verificado (D1)

No app real (build de debug e de release, sem mexer no mouse: os comandos chamados pela janela do dock), com Chrome e Claude já abertos e Bloco de notas, Calculadora e Mapa de Caracteres abertos pelo teste, todos fixados:

- o indicador aparece nos cinco programas abertos e não no Paint, que não está; cada um casou por um dos dois caminhos (AppID da janela ou alvo do atalho);
- ativar o Bloco de notas traz a janela para a frente (indicador "ativo"); ativar de novo, estando nele, **minimiza**; de novo, **restaura**; ativar a Calculadora passa o primeiro plano para ela;
- fechar o Mapa de Caracteres pelo botão de fechar apaga a marca na hora; **encerrar o Bloco de notas à força** apaga em até 9 s (a checagem de 5 s);
- ativar um programa fixado que não está aberto devolve `false` (e o dock o abre);
- **memória e CPU** (release, com Chrome, Claude e Calculadora fixados e abertos, 25 s depois de abrir): 210 MB privados (202 MB 30 s depois), CPU 0,07 % do total dos 16 processadores; a meta do roadmap é cerca de 220 MB e menos de 1 %. A memória subiu perto de 20 MB em relação à medição do MVP (174-187 MB), que tinha poucos programas fixados; parte é dos ícones dos três programas, parte do vigia, e a margem até a meta ficou pequena.

**Não verificado:** o clique do mouse de verdade (a permissão de trazer uma janela para a frente depende de o Windows ter dado a entrada ao DockMax; o teste usou o comando), vários monitores, janelas em outra área de trabalho virtual e a primeira janela de um programa que abre devagar.
