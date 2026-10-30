# Spec 0019 — Fase G: extras

Fase G da [spec 0004](0004-paridade-primodock.md) (§6), depois da fase F ([spec 0017](0017-fase-f-widgets.md), concluída em 2026-10-29).

**Status:** em andamento: G1 (idioma inglês, #75), G2 (tour, #79), G4 (ícones retrô, #80), G5 (estudo do Linux, #76) e G6 (nome e logo, #77) entregues; o renome para **Dockyard** (R1) entrou em 2026-10-30, com o repositório renomeado para `maxeinstein-dev/Dockyard`. A G3 (atualização automática) foi implementada em 2026-10-30 e estreia na 0.4.0. Ordem decidida pelo autor em 2026-10-29: tour, pacote de ícones, nome e logo, estudo do Linux e, por último, atualização automática.

## 1. Decisões do autor (2026-10-29)

O que entra na fase, no lugar da lista da spec 0004 (mascote, onboarding, configurações rápidas e idioma):

| Item | Decisão |
|---|---|
| Configurações rápidas | Já existem (fase D): bateria, rede e volume no fim do dock abrem o painel do Windows |
| Mascote | **Fica de fora** |
| **Idioma inglês** | Entra, e é o primeiro recorte |
| **Tour de boas-vindas** | Entra |
| **Atualização automática** | Entra |
| **Pacotes de ícones** | Entra: começa por ícones retrô / 8-bit |
| **Linux** | Entra como estudo: como fazer o DockMax funcionar no Linux |
| **Nome e logo** | Talvez: um nome e uma logo mais "comerciáveis" que DockMax. Decidido: **Dockyard** (G6) |

*Nota: Canarinho removido em 2026-10-30, a pedido do autor. Um `settings.json` antigo com `"style": "canary"` abre no tema padrão (Nativo), sem erro e com os outros ajustes intactos.*

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **G1. Idioma inglês** | Seletor **Idioma** em Configurações (Sistema, Português, English); toda a interface em inglês, do front (`src/i18n/en.ts`) e do Rust (`i18n.rs`: menus, nomes do dock, lançador); datas e números no formato do idioma. "Sistema" segue o idioma de exibição do Windows: português se for um, inglês nos outros. A troca vale na hora, sem reabrir o app. |
| **G2. Tour de boas-vindas** | Uma janela pequena com 6 passos (boas-vindas, apps, espaços, widgets, lançador, pronto), com Próximo, Voltar e Pular; no fim, Começar ou Abrir Configurações. Abre sozinha na primeira execução e em Sobre › Ver o tour. |
| G3. Atualização automática | Releases do GitHub do repositório (público), com o updater do Tauri; detalhes no §5. |
| **G4. Pacote de ícones retrô / 8-bit** | O primeiro pacote próprio, **Retrô 8-bit**: 43 ícones em pixel art 16×16 na paleta do PICO-8 (o dock do autor e uns 35 apps comuns, mais um cartucho de reserva), no formato do B4a ([spec 0007](0007-fase-b4-icones.md) §3). Vem embutido e o app o copia para `IconPacks` ao abrir; o usuário o escolhe em Aparência › Ícones dos apps. |
| **G4.1. Pacote Esmalte** | Segundo pacote embutido, **Esmalte**: os mesmos 43 apps em ladrilhos como os do dock (superelipse, esmalte de dois tons da mesma matiz, brilho no topo, borda de luz e sombra) com um glifo branco de traço por app, desenhado no projeto. Gerado como SVG e rasterizado em 128 px. Ver §7. |
| **G4.2. Pacote Traço** | Terceiro pacote embutido, **Traço**: os mesmos glifos do Esmalte sozinhos, em linha fina (traço 1,5 numa caixa de 24, com 81% da imagem), no estilo dos glifos de linha do app. Uma tinta por tema, pelo mapa `dark` do formato: `#555A66` no claro e `#A9AEB9` no escuro. Ver §7. |
| **G4.3. Pacote Clássico 98** | Quarto pacote embutido, **Clássico 98**: os mesmos 43 apps em pixel art 32×32 na paleta do Windows 95/98 (as 16 cores e o cinza-claro do bisel), cada um num botão com o bisel do 98 e o desenho com sombra de 1 px. Fonte em texto e gerador, como o Retrô. Ver §7. |
| **G4.4. Pacote Círculo** | Disco chapado na cor do app, glifo branco e sombra curta, como os temas de ícones redondos. Ver §7. |
| **G4.5. Pacote Contorno colorido** | Os glifos sozinhos, cada um na matiz do app: luz 38 no tema claro e 68 no escuro, pelo mapa `dark`. Ver §7. |
| **G4.6. Pacote Tom sobre tom** | Placa e glifo da mesma matiz: placa clara e glifo escuro no tema claro, o contrário no escuro. Ver §7. |
| **G4.7. Pacote Vidro** | Placa translúcida tingida da matiz, com borda de luz, brilho e glifo branco; mais opaca no tema claro. Ver §7. |
| **G4.8. Pacote Neon** | O glifo como tubo de neon na cor do app, com brilho; sobre uma placa escura no tema claro, solto no escuro. Ver §7. |
| **G4.9. Pacote Argila** | Placa fofa na cor do app, com luz e sombra por dentro, sombra macia e glifo branco grosso em relevo. Ver §7. |
| **G4.10. Pacote Fósforo** | Pixel art 32×32 de monitor de fósforo verde: moldura com bisel, linhas de varredura e o glifo aceso com halo. Ver §7. |
| **G4.11. Pacote Game Boy** | Pixel art 16×16 em quatro verdes, a partir dos desenhos do Retrô. Ver §7. |
| **G5. Linux** | Feito: [estudo 0001](../estudos/0001-linux.md). Recomenda não portar agora; se o Linux virar meta, separar o Windows atrás de traits e fazer um dock para KDE Plasma e wlroots com layer-shell. GNOME/Zorin só com uma extensão do Shell, à parte. Decisões do autor no §6 do estudo. |
| **G6. Nome e logo** | **Decidido em 2026-10-29:** o nome passa a ser **Dockyard**, com o ícone do lançador saindo da barra em esmalte cobalto ([marca 0001](../marca/0001-proposta.md)). **R1 feito em 2026-10-30:** nome, exe, ícone, textos, iniciar com o Windows e repositório; o identificador e as pastas de dados continuam `dockmax`, sem migração. |

## 3. G1 em detalhe

- `settings.json` ganha `language`: `"system"` (padrão), `"pt-BR"` ou `"en"`. Um arquivo antigo, sem o campo, fica em "Sistema".
- Rust: `i18n::set_language` escolhe os textos ao abrir e a cada mudança das configurações; com "Sistema", lê `GetUserDefaultUILanguage`. Os menus nativos e os nomes do dock são montados a cada vez, então mudam na próxima abertura ou pintura.
- Front: `useT` acompanha o idioma (o `settings-changed` chega a todas as janelas); `activeLocale()` dá o idioma para `Intl` e `toLocaleString`. O `<html lang>` acompanha.
- O contrato não muda: `en.ts` tem o tipo `Messages` do PT-BR, e o Rust tem a mesma struct `Messages`; uma chave faltando não compila.

## 4. G2 em detalhe

- Decisões do autor (2026-10-29): **janela com passos** (não balões apontando no dock) e **abre na primeira execução e em Sobre**.
- `settings.json` ganha `tourShown`. Uma instalação nova começa sem ele e o tour abre uma vez; um arquivo de antes do tour, sem o campo, conta como mostrado, para não abrir para quem já usa. O Rust marca ao abrir, então fechar no meio não traz o tour de volta.
- A janela `tour` (`tour.html`, `src/tour/`) tem a sua capability: ler as configurações (tema, idioma e o atalho do lançador, que o passo 5 cita) e `finish_tour`, que fecha a janela e, se pedido, abre Configurações. "Ver o tour" (`open_tour`) fica no Sobre do painel e de Configurações.
- Teclado: setas passam os passos, Esc pula; o título de cada passo recebe o foco, para o leitor de tela ler.

## 5. G4 em detalhe

Decisões do autor (2026-10-29): lista grande de apps mais os do dock dele; paleta do PICO-8; grade 16×16; embutido, instalando sozinho. Os widgets e os glyphs do dock ficam fora: o formato do B4a só veste programas.

- **Fonte:** `src-tauri/icon-packs/fonte/retro-8bit.txt`, um desenho por app em 16 linhas de 16 caracteres (`0`–`f` são as 16 cores do PICO-8, `.` é transparente), com as chaves do `pack.json` em `nomes:`. `gerar.py` (só biblioteca padrão) escreve `src-tauri/icon-packs/retro-8bit/` — o `pack.json` e um PNG de 128 px por desenho, cada pixel ampliado 8× sem suavização — e a folha `fonte/retro-8bit-previa.png`. Para estender: desenhar, rodar `python gerar.py retro-8bit` e somar o PNG à lista de `persistence/bundled_packs.rs`.
- **Chaves:** o nome que o Windows mostra, em português e em inglês, e o nome do executável quando difere (`Prompt de Comando`, `Command Prompt`, `cmd`). `nomes: reserva` é o `fallback`: um cartucho para o programa que nenhum nome cobre.
- **Instalação:** os arquivos vão no binário (`include_bytes!`). Ao abrir, antes do dock ler um ícone, o app copia o pacote para `IconPacks\retro-8bit\` e anota `retro-8bit <versão>` em `IconPacks\.embutidos`. Só copia de novo quando a versão do `pack.json` muda: um pacote que o usuário apagou fica apagado até a próxima versão, e uma edição dele só é sobrescrita por uma versão nova. Uma pasta do pacote que é link fica como está. O pacote nunca se liga sozinho.
- **Testes** (`persistence::bundled_packs`): a lista embutida é a do `pack.json` e a da pasta; o pacote instalado passa pelo `icon_packs::scan` e cada imagem pelo `read_image`; cobre os sete apps do dock do autor e cai no cartucho para o resto; nenhuma chave repete outra depois de normalizada; a cópia é uma por versão; e cada PNG é, pixel a pixel, o desenho da fonte.
- **Verificado** (2026-10-29, build de teste em release, dados numa pasta temporária com uma cópia do banco do autor): ao abrir, `IconPacks` ganhou `retro-8bit` (43 PNGs) e `.embutidos` com `retro-8bit 1.0.0`; em Aparência › Ícones dos apps o seletor mostrou "Retrô 8-bit · veste 7 de 7 apps", e escolhido, o `settings.json` gravou `dock.iconPack: "retro-8bit"` e o dock trocou os ícones dos fixados e dos abertos (Claude, Chrome de outro perfil) sem reabrir. Antes e depois em [`docs/fotos/g4-dock-retro-8bit.png`](../fotos/g4-dock-retro-8bit.png).

## 6. G3 em detalhe

Decisões do autor (2026-10-30):

- **Onde:** Releases do GitHub de `maxeinstein-dev/Dockyard`, que é público. O app lê `releases/latest/download/latest.json`.
- **Como atualiza:** avisa e o usuário confirma. Procura ao abrir e a cada 6 horas; com versão nova, mostra uma notificação e um aviso em Sobre com **Atualizar**. Só baixa, instala e reabre depois do clique.
- **Como publica:** uma tag `vX.Y.Z` dispara a CI, que compila o instalador NSIS, assina e publica o Release com o `latest.json`.
- **Chave de assinatura:** nos secrets do repositório (`TAURI_SIGNING_PRIVATE_KEY` e `..._PASSWORD`), com cópia em `%USERPROFILE%\.tauri\dockyard.key` e a senha no Gerenciador de Credenciais (`Dockyard/updater-key-password`). Sem a chave, quem já instalou não recebe mais atualizações.
- **Instalador:** português e inglês, seguindo o idioma do Windows.

### Implementação (2026-10-30)

- **Configuração:** `tauri-plugin-updater` com `plugins.updater` no `tauri.conf.json` (o `latest.json` do último Release, a chave pública da `dockyard.key` e `windows.installMode: "passive"`) e `bundle.createUpdaterArtifacts`. O instalador NSIS tem `PortugueseBR` e `English` sem seletor: segue o idioma do Windows. A versão sobe para **0.4.0**, a primeira com atualização.
- **Rust:** `domain::update` tem as regras sem rede (o estado — nenhuma, disponível, baixando, instalando —, o aviso uma vez por versão e o intervalo); `windows::update` procura 15 s depois de abrir e a cada 6 h numa thread, como o Clima. Só no build de release: o de debug não procura sozinho, e o de teste (`e2e`) nem registra o plugin. Sem rede, só um `warn` no log. Uma versão nova mostra uma vez a notificação do Windows "Dockyard X.Y.Z disponível" (texto em `i18n.rs`; sem clique, o toast só avisa).
- **Sobre** (o do painel e o de Configurações, o mesmo `AboutPanel`): "Versão nova: X.Y.Z" com **Atualizar**; sem versão nova, **Procurar atualizações**, que responde "Você está na versão mais recente" ou o erro. O evento `update-changed` leva o estado e o progresso do download a todas as janelas.
- **Commands:** `update_status`, `check_for_update` e `install_update`, nas capabilities do painel e de Configurações; os commands do próprio plugin não estão em nenhuma.
- **Instalar:** baixa (assinatura conferida pelo plugin), depois roda o `install` na thread principal. No Windows o `install` termina o processo sem passar pelo `RunEvent::Exit`; o `on_before_exit` dele chama `release_on_exit`, a mesma limpeza do Sair (bandeja, faixa do dock, **barra do Windows de volta**, janelas escondidas pelos espaços, dock e lançador soltos, checkpoint do banco), uma vez só. Se o instalador não abrir depois disso, o app se reinicia. O instalador passivo recebe `/P /UPDATE /R` e abre o Dockyard no fim.
- **Publicar:** `.github/workflows/release.yml`, numa tag `v*`: um job confere que a tag bate com `tauri.conf.json`, `Cargo.toml` e `package.json`, e o `tauri-action` compila, assina e publica o Release com o `latest.json`.
- **Testes:** `domain::update` (estado, aviso único, intervalo), o progresso de `windows::update`, as guardas (`release_on_exit` no `RunEvent::Exit` com o checkpoint; o updater só instala depois dele, na thread principal) e o `AboutPanel` com e sem versão nova.

## 7. Pacotes Esmalte, Traço e Clássico 98 em detalhe

Decisões do autor (2026-10-30): mais três pacotes embutidos, um PR cada, na ordem Esmalte, Traço e Clássico 98; cobrem os mesmos apps do Retrô; desenho próprio, nada de logo de marca.

- **Glifos:** `src-tauri/icon-packs/fonte/glifos.txt`, comum aos pacotes vetoriais: por app, o arquivo (`@id`), as chaves do `pack.json` (`nomes:`, as mesmas do Retrô; `reserva` é o fallback, uma grade de quatro quadrados), a matiz do ladrilho (`esmalte: h s l`, no registro do DESIGN.md: saturação 14–48, luminosidade 34–46) e os caminhos SVG numa caixa de 24 px, para traço de 1,75 com pontas e junções redondas, como os glifos do dock. Cada glifo é a ideia do programa (pasta, globo, chama, escudo, envelope…), não o logo dele.
- **Gerador:** `fonte/vetor.py <id>` (só biblioteca padrão) lê os glifos e `fonte/<id>.txt` (metadados e estilo), escreve um SVG por app em `fonte/<id>-svg/` e o rasteriza em `src-tauri/icon-packs/<id>/apps/*.png`, com a folha `fonte/<id>-previa.png`. O rasterizador é do próprio script (amostragem 4×4 por pixel) e cobre o que esses SVGs usam; conferido lado a lado com o SVG renderizado pelo navegador.
- **Tamanho:** um PNG por app (o formato do B4a não tem variantes por tamanho), de 128 px como o Retrô: o dock desenha os ícones de 32 a 96 px lógicos, ampliados até 1,4× e na escala do monitor, com interpolação linear, que reduz bem a partir de 128.
- **Esmalte** (`esmalte.txt`): o ladrilho ocupa 112 dos 128 px (de (8, 4)), deixando espaço para a sombra embaixo; a superelipse, os dois tons (+7 e −5 de luminosidade), o brilho radial, a borda de luz e as duas sombras são os de `native_dock/paint.rs`, na escala de um ladrilho de 48 px. O glifo ocupa 54% do ladrilho, em branco.
- **Traço** (`traco.txt`): o glifo sozinho, sem ladrilho, com a caixa de 24 em 104 dos 128 px (81%) e traço de 1,5, mais fino que o 1,75 dos glifos sobre esmalte, porque aqui o glifo é o ícone inteiro. Na versão 1.0.0 o formato tinha uma imagem por app, e a tinta era um cinza neutro único (`#7C818C`, 3,6:1 e 3,8:1). Desde a 1.1.0, com o mapa `dark` do formato ([spec 0007](0007-fase-b4-icones.md) §3.1), cada tema tem a sua: grafite `#555A66` no claro (6,5:1 na porcelana, em `apps/`) e cinza-claro `#A9AEB9` no escuro (6,7:1 no grafite, em `escuro/`). Em `vetor.py`, uma chave `<nome>-escuro` no `.txt` do pacote dá o valor do tema escuro: com uma, o gerador escreve `escuro/*.png`, os SVGs em `fonte/<id>-escuro-svg/` e o mapa `dark`, e a folha de prévia mostra o escuro no fundo escuro.
- **Clássico 98** (`classico-98.txt`): pixel art 32×32 pelo `gerar.py` do Retrô, que agora lê `pack.grid` (o lado do desenho; 16 se faltar, e o Retrô sai igual). Cada pixel vira 4×4, para o PNG de 128 px. A paleta são as 16 cores do Windows 95/98 e o `#DFDFDF` do bisel. Cada ícone é um botão do 98: branco em cima e à esquerda, cinza-claro por dentro, cinza e preto embaixo e à direita, face `#C0C0C0`. Dentro dele vai o desenho do app, com contorno, preenchimento e detalhes em cores VGA e sombra cinza de 1 px. Os desenhos partiram dos glifos (o contorno de cada glifo recebe o preenchimento e as outras linhas viram detalhe) e são editados no texto.
- **Estilos de camadas** (2026-10-30, decisão do autor: "pode seguir com todos esses", depois da lista de estilos comuns em docks, temas de ícones e launchers). `fonte/camadas.py` descreve cada estilo como uma lista de camadas: forma (a placa em superelipse, o disco ou o glifo), tinta (cor, gradiente vertical ou o brilho do esmalte), opacidade, deslocamento, uma forma subtraída (para bordas de luz e de sombra), desfoque (três passadas de média móvel; `feGaussianBlur` no SVG) e recorte pela placa. O PNG e o SVG saem da mesma lista. As cores vêm da matiz de cada glifo (`esmalte:`). As chaves `-escuro` de cada `.txt` dão o tema escuro, como no Traço.
  - **Círculo** (`circulo.txt`): disco de 112 px na matiz do glifo, com saturação ×1,35 (até 60) e luz +4; sombra curta embaixo; glifo branco com 50% do disco. Um tema só.
  - **Contorno colorido** (`contorno.txt`): o estilo do Traço com `cor: matiz`, saturação 58, luz 38 no claro e 68 no escuro, traço 1,75. Todas as 43 matizes passam de 3:1 na superfície do dock de cada tema (teste).
  - **Tom sobre tom** (`tom-sobre-tom.txt`): placa na matiz com luz 82 e glifo com luz 30 no claro; placa 24 e glifo 80 no escuro.
  - **Vidro** (`vidro.txt`): placa na matiz (luz 52) com opacidade 0,78 no claro e 0,55 no escuro, véu branco em gradiente, o brilho do esmalte, borda de luz no topo e glifo branco com sombra suave. O fundo do dock aparece pela placa.
  - **Neon** (`neon.txt`): o glifo na matiz com saturação 95 e luz 58, dois brilhos desfocados e um núcleo claro mais fino. No claro fica sobre uma placa grafite (caixa 62%); no escuro fica solto (caixa 78%), com o brilho em alfa.
  - **Argila** (`argila.txt`): placa na matiz com luz +14, sombra macia no chão, luz por dentro em cima e à esquerda, sombra por dentro embaixo e à direita, e glifo branco de traço 2,4 com sombra. Um tema só.
- **Fósforo** (`fosforo.txt`, `gerar.py`, grade 32, ×4): moldura de 3 px com bisel (luz em cima e à esquerda, sombra embaixo e à direita) e cantos cortados. A tela tem linhas de varredura alternadas (`0` e `1`), o glifo aceso em `#3CFF6E` (`4`) e o halo: `3` ao lado do traço e `2` na diagonal. Os desenhos partiram dos glifos de `glifos.txt` rasterizados em 22 px e se editam no texto.
- **Game Boy** (`game-boy.txt`, `gerar.py`, grade 16, ×8): os desenhos do Retrô em quatro verdes (`#081820`, `#346856`, `#88C070`, `#E0F8D0`). É a paleta dos emuladores, mais espaçada que a do aparelho, para os detalhes não sumirem. Em cada desenho, as cores quase pretas viram `0` e as outras se espalham de `1` a `3` na ordem da luminância. A reserva é o cartucho, como no Retrô.
- **Testes** (`persistence::bundled_packs`): os do Retrô valem para todo pacote embutido (a lista é a do `pack.json` e a da pasta, o pacote instalado passa pelo `scan` e pelo `read_image`, cobre o dock do autor, cai na reserva, nenhuma chave repete outra, cada pacote é anotado uma vez em `.embutidos`); cada PNG do Retrô e do Clássico 98 é, pixel a pixel, o desenho da fonte, e todo desenho do Clássico tem o bisel do 98; cada PNG do Esmalte é o glifo da fonte: transparente fora da superelipse, com a cor do esmalte da fonte num ponto do ladrilho (conferido com sabotagem: outra matiz na fonte faz o teste falhar), o glifo branco, e o SVG ao lado. Cada pacote de camadas tem um PNG de 128 px por glifo em cada tema (com o SVG de cada um), o canto livre e o desenho no meio, e o mapa `dark` completo quando tem tema escuro. O Contorno tem, em cada pixel, a matiz do glifo na luz do tema, com 3:1 ou mais na superfície do dock (conferido com sabotagem: com luz 30 no escuro, o teste falha em 2,16:1). O Fósforo e o Game Boy passam pelo teste pixel a pixel da fonte, como o Retrô e o Clássico; o Game Boy tem quatro verdes, cada um bem mais claro que o anterior, e toda tela do Fósforo tem só linhas de varredura e fósforo, com o glifo aceso. Cada PNG do Traço tem só a cor da fonte, variando a opacidade, e essa cor tem contraste de 3:1 ou mais nos dois temas.
