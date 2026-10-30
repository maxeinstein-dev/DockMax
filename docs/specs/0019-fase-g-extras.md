# Spec 0019 — Fase G: extras

Fase G da [spec 0004](0004-paridade-primodock.md) (§6), depois da fase F ([spec 0017](0017-fase-f-widgets.md), concluída em 2026-10-29).

**Status:** em andamento: G1 (idioma inglês, #75) entregue; G2 (tour) em curso. Ordem decidida pelo autor em 2026-10-29: tour, pacote de ícones, nome e logo, estudo do Linux e, por último, atualização automática.

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
| G3. Atualização automática | A definir (onde publicar as versões). |
| **G4. Pacote de ícones retrô / 8-bit** | O primeiro pacote próprio, **Retrô 8-bit**: 43 ícones em pixel art 16×16 na paleta do PICO-8 (o dock do autor e uns 35 apps comuns, mais um cartucho de reserva), no formato do B4a ([spec 0007](0007-fase-b4-icones.md) §3). Vem embutido e o app o copia para `IconPacks` ao abrir; o usuário o escolhe em Aparência › Ícones dos apps. |
| **G5. Linux** | Feito: [estudo 0001](../estudos/0001-linux.md). Recomenda não portar agora; se o Linux virar meta, separar o Windows atrás de traits e fazer um dock para KDE Plasma e wlroots com layer-shell. GNOME/Zorin só com uma extensão do Shell, à parte. Decisões do autor no §6 do estudo. |
| **G6. Nome e logo** | **Decidido em 2026-10-29:** o nome passa a ser **Dockyard**, com o ícone do lançador saindo da barra em esmalte cobalto ([marca 0001](../marca/0001-proposta.md)). Só proposta e SVGs; o renome no código é um recorte futuro, com os passos na proposta. |

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
