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

## 7. Pacotes vetoriais em detalhe

Decisões do autor (2026-10-30): mais três pacotes embutidos, um PR cada, na ordem Esmalte, Traço e Clássico 98; cobrem os mesmos apps do Retrô; desenho próprio, nada de logo de marca.

- **Glifos:** `src-tauri/icon-packs/fonte/glifos.txt`, comum aos pacotes vetoriais: por app, o arquivo (`@id`), as chaves do `pack.json` (`nomes:`, as mesmas do Retrô; `reserva` é o fallback, uma grade de quatro quadrados), a matiz do ladrilho (`esmalte: h s l`, no registro do DESIGN.md: saturação 14–48, luminosidade 34–46) e os caminhos SVG numa caixa de 24 px, para traço de 1,75 com pontas e junções redondas, como os glifos do dock. Cada glifo é a ideia do programa (pasta, globo, chama, escudo, envelope…), não o logo dele.
- **Gerador:** `fonte/vetor.py <id>` (só biblioteca padrão) lê os glifos e `fonte/<id>.txt` (metadados e estilo), escreve um SVG por app em `fonte/<id>-svg/` e o rasteriza em `src-tauri/icon-packs/<id>/apps/*.png`, com a folha `fonte/<id>-previa.png`. O rasterizador é do próprio script (amostragem 4×4 por pixel) e cobre o que esses SVGs usam; conferido lado a lado com o SVG renderizado pelo navegador.
- **Tamanho:** um PNG por app (o formato do B4a não tem variantes por tamanho), de 128 px como o Retrô: o dock desenha os ícones de 32 a 96 px lógicos, ampliados até 1,4× e na escala do monitor, com interpolação linear, que reduz bem a partir de 128.
- **Esmalte** (`esmalte.txt`): o ladrilho ocupa 112 dos 128 px (de (8, 4)), deixando espaço para a sombra embaixo; a superelipse, os dois tons (+7 e −5 de luminosidade), o brilho radial, a borda de luz e as duas sombras são os de `native_dock/paint.rs`, na escala de um ladrilho de 48 px. O glifo ocupa 54% do ladrilho, em branco.
- **Testes** (`persistence::bundled_packs`): os do Retrô valem para todo pacote embutido (a lista é a do `pack.json` e a da pasta, o pacote instalado passa pelo `scan` e pelo `read_image`, cobre o dock do autor, cai na reserva, nenhuma chave repete outra, cada pacote é anotado uma vez em `.embutidos`); e cada PNG do Esmalte é o glifo da fonte: transparente fora da superelipse, com a cor do esmalte da fonte num ponto do ladrilho (conferido com sabotagem: outra matiz na fonte faz o teste falhar), o glifo branco, e o SVG ao lado.
