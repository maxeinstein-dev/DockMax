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

## 3.1 D2 em detalhe: os programas abertos que não estão fixados

- **Onde:** depois dos widgets e antes dos itens do sistema, com um separador, na ordem em que **apareceram** (a ordem não muda quando a pilha de janelas muda; um programa novo vai para o fim). A configuração `dock.showOpenApps` ("Mostrar apps abertos", ligada por padrão) os mostra ou esconde; desligada, o Rust nem os procura. A ordem é guardada enquanto estão escondidos.
- **Quais:** as janelas com botão que nenhum programa fixado reivindica, agrupadas pelo AppID que declaram ou, sem AppID, pelo executável: **um item por AppID, como a barra do Windows** (os perfis de um navegador, cada um com o seu, ficam em itens separados).
- **Nome e ícone:** o item da pasta Aplicativos do mesmo AppID; senão, o programa da pasta Aplicativos que roda o mesmo executável (lido uma vez, cerca de um segundo); senão, o próprio executável e o nome do arquivo (o mesmo caminho de um programa solto no dock). Janelas sem nenhuma dessas coisas não aparecem.
- **O que a página recebe:** `{ key, appId, name, count, active }`. A `key` é opaca (16 dígitos hexadecimais, do hash do programa): o clique envia `activate_open(key)` e o Rust acha as janelas. A página não vê janela nem caminho de executável de um programa da pasta Aplicativos; para um executável solto o `appId` é o caminho, como o de um programa fixado por soltar. O `list_open_apps` e o evento `open-windows-changed` passam a levar `{ apps, others }`.
- **Clique:** o mesmo dos fixados (traz, minimiza, próxima janela). Não arrasta e ainda não tem menu (D3).
- **Botão do meio** (a roda), nos fixados e nestes: abre uma **nova instância** do programa, como a barra do Windows (um programa de instância única só traz a janela que já tem). É a configuração `dock.middleClick` ("Botão do meio", em Configurações): `newInstance` (padrão) ou `none`; mais ações (como "Fechar janela") entram com o menu do D3. Para um programa aberto que não está fixado, o dock envia `launch_open(key)` (a mesma chave opaca) e o Rust abre o programa pelo id que descreveu. A página impede o modo de rolagem do navegador no botão do meio.
- **Indicador:** o mesmo ponto ou traço, sempre visível nesses itens (eles só existem enquanto estão abertos).

## 3.2 D3 em detalhe: o menu dos programas abertos

- **Programa aberto que não está fixado** (botão direito no item): um menu no painel, com **Nova instância**, **Fechar janela** (ou **Fechar todas as janelas**, se tem mais de uma) e **Fixar no dock**. O painel é `{ "open": "<chave>" }` (nova variante de `PanelId`, `Panel::Open` no Rust): o `toggle_panel` só o abre para uma chave de 16 dígitos hexadecimais que **está aberta agora**; se o programa fecha com o menu aberto, o painel fecha.
- **Programa fixado que está aberto:** o menu que já existia (a Jump List, Abrir, Mostrar no Explorador, Remover do dock) ganha **Fechar janela / Fechar todas as janelas** enquanto ele tem janela.
- **Fechar:** `WM_CLOSE` em cada janela do programa, como o botão de fechar de cada uma: o programa pode perguntar se quer salvar ou recusar. Nada é encerrado à força. Comandos `close_app(id do item fixado)` e `close_open(chave)`.
- **Fixar:** `pin_open(chave)`. O id e o nome que se fixam são os que o Rust já tinha descrito ao mostrar o programa (a página só nomeia a chave opaca); depois de fixado, o item passa para os fixados.
- **Capacidades:** os cinco comandos novos (`launch_open`, `close_app`, `close_open`, `pin_open` e a leitura `list_open_apps`) ficam na janela do painel, pinados no guarda; o dock só ganhou o que já tinha.

## 3.3 Os perfis do Chrome

- **Separados, como no Windows:** cada perfil do Chrome tem um AppID próprio (`Chrome` para o padrão, `Chrome.UserData.<pasta sem espaços>` para os outros) e é um item. Um programa fixado não reivindica, só pelo executável, uma janela cujo AppID **estende** o dele (`Chrome.UserData.Profile1` de `Chrome`): o Chrome fixado é o do perfil padrão, e os outros perfis aparecem como abertos não fixados.
- **Nome:** o rótulo leva o nome do perfil ("Google Chrome (Trabalho)").
- **Foto:** como o selo que o Chrome põe no botão da barra do Windows (que nenhum outro programa consegue ler), o dock desenha a **foto da conta** do perfil no canto do ícone. Ela vem do próprio Chrome, só leitura: a lista de perfis em `%LOCALAPPDATA%\Google\Chrome\User Data\Local State` (`profile.info_cache`: nome e arquivo da foto) e o arquivo da foto na pasta do perfil. O Rust confere nome de arquivo, tamanho e cabeçalho PNG, reduz a foto a 64 px e a entrega por `open_badge(chave)` como `data:image/png`; a página sabe só que há selo (`badge`). Nomes e fotos não vão para o log. Um perfil sem foto da conta fica sem selo.
- **Só o Chrome estável** por enquanto (Edge, Beta e Canary usam outras pastas e AppIDs).

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

## 6. Verificado (D2)

No app real (build de debug, sem mouse), com Calculadora e Mapa de Caracteres abertos pelo teste e o Terminal, o Chrome (dois perfis), o Claude e o WhatsApp que já estavam abertos, nada fixado:

- aparecem seis itens com ícone real, depois dos widgets e antes de Configurações; o Chrome com **dois perfis são dois itens**, como na barra do Windows;
- ativar a Calculadora pela chave traz a janela e o indicador "ativo" passa para ela; uma chave com formato errado é recusada (`notFound`) e uma chave desconhecida devolve `false`;
- fechar o Mapa de Caracteres pelo botão de fechar tira o item na hora;
- desligar "Mostrar apps abertos" tira todos; ligar de novo os traz de volta **na mesma ordem**.
- **memória e CPU** (release, nada fixado, cinco programas abertos mostrados no dock, 25 s depois de abrir): 201 MB privados (198 MB 30 s depois) e 0,04 % do total dos 16 processadores. A meta do roadmap é cerca de 220 MB e menos de 1 %; o D2 custa pouco, mas a margem continua pequena.
- **Não verificado:** o clique de mouse real, vários monitores e áreas de trabalho virtuais (como no D1).
- **botão do meio:** no Mapa de Caracteres aberto, o evento do botão do meio abriu uma segunda instância (1 → 2 processos); com "Botão do meio" em "Nada", não abriu outra (continuou 2). O botão físico do mouse não foi usado: o evento foi enviado à página.

## 7. Verificado (D3)

No app real (build de debug, sem mouse: os comandos pela janela do dock e do painel), com a Calculadora e o Mapa de Caracteres abertos pelo teste:

- o menu do Mapa de Caracteres abre com "Nova instância", "Fechar janela" e "Fixar no dock"; uma chave inválida é recusada;
- "Fechar janela" fecha o programa (1 → 0 processos) e o item sai do dock;
- "Fixar no dock" na Calculadora a põe entre os fixados (com o indicador de aberto) e a tira dos abertos não fixados; chave inválida ou desconhecida é recusada (`notFound`);
- o menu da Calculadora fixada traz "Fechar janela" junto da Jump List, e escolhê-lo a fecha.

**Não verificado:** o clique do botão direito do mouse de verdade (o painel foi aberto pelo comando), o rótulo do menu com vários monitores e o comportamento de um programa que pergunta se quer salvar ao fechar.

## 8. Verificado (perfis do Chrome)

No app real, com dois perfis do Chrome abertos e nada fixado: dois itens, "Google Chrome (Max Pessoal)" e "Google Chrome (Max Alfama)", cada um com a foto do perfil (64 px) no canto do ícone; os outros programas sem selo.
