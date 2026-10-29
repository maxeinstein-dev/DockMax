# Spec 0019 — Fase G: extras

Fase G da [spec 0004](0004-paridade-primodock.md) (§6), depois da fase F ([spec 0017](0017-fase-f-widgets.md), concluída em 2026-10-29).

**Status:** em andamento: G1 (idioma inglês) em curso.

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
| **Nome e logo** | Talvez: um nome e uma logo mais "comerciáveis" que DockMax |

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **G1. Idioma inglês** | Seletor **Idioma** em Configurações (Sistema, Português, English); toda a interface em inglês, do front (`src/i18n/en.ts`) e do Rust (`i18n.rs`: menus, nomes do dock, lançador); datas e números no formato do idioma. "Sistema" segue o idioma de exibição do Windows: português se for um, inglês nos outros. A troca vale na hora, sem reabrir o app. |
| G2. Tour de boas-vindas | A definir. |
| G3. Atualização automática | A definir (onde publicar as versões). |
| G4. Pacote de ícones retrô / 8-bit | A definir. |
| G5. Linux | Estudo do que muda (o dock nativo é Win32/Direct2D). |
| G6. Nome e logo | A definir com o autor. |

## 3. G1 em detalhe

- `settings.json` ganha `language`: `"system"` (padrão), `"pt-BR"` ou `"en"`. Um arquivo antigo, sem o campo, fica em "Sistema".
- Rust: `i18n::set_language` escolhe os textos ao abrir e a cada mudança das configurações; com "Sistema", lê `GetUserDefaultUILanguage`. Os menus nativos e os nomes do dock são montados a cada vez, então mudam na próxima abertura ou pintura.
- Front: `useT` acompanha o idioma (o `settings-changed` chega a todas as janelas); `activeLocale()` dá o idioma para `Intl` e `toLocaleString`. O `<html lang>` acompanha.
- O contrato não muda: `en.ts` tem o tipo `Messages` do PT-BR, e o Rust tem a mesma struct `Messages`; uma chave faltando não compila.
