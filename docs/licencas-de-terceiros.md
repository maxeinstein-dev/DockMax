# Licenças das dependências

Verificado em 2026-10-26, na versão 0.3.0. O DockMax em si tem licença de [uso pessoal](../LICENSE); as dependências abaixo têm as suas.

## Resultado

**Nenhuma dependência de produção tem licença GPL, AGPL ou outra que obrigue a abrir o código do DockMax.**

### Frontend (pnpm, só o que vai no aplicativo)

| Pacote | Licença |
|---|---|
| `react`, `react-dom`, `scheduler` | MIT |
| `@tauri-apps/api` | Apache-2.0 OR MIT |
| `@tauri-apps/plugin-log` | MIT OR Apache-2.0 |

As ferramentas de desenvolvimento (Vite, Vitest, ESLint, WebdriverIO…) não vão no instalador.

### Rust (224 pacotes da árvore normal, alvo `x86_64-pc-windows-msvc`)

| Licença | Pacotes |
|---|---|
| MIT, Apache-2.0 ou as duas (em qualquer ordem) | 182 |
| Unicode-3.0 (tabelas de caracteres, `icu_*`) | 19 |
| Unlicense ou MIT | 9 |
| BSD-3-Clause (sozinha ou com MIT): `alloc-no-stdlib`, `alloc-stdlib`, `brotli`, `brotli-decompressor` | 4 |
| **MPL-2.0**: `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`, `selectors` | 5 |
| Zlib, CC0-1.0, 0BSD (sempre como alternativa a MIT ou Apache-2.0) | 5 |

A única licença com alguma exigência a mais é a **MPL-2.0**: ela vale **por arquivo**. Quem muda um arquivo dessas bibliotecas precisa publicar a mudança; usar a biblioteca sem mudá-la, como o DockMax faz, não obriga a nada além de manter o aviso de licença e deixar o código delas disponível (ele está no crates.io).

O WebView2 é um componente do Windows (runtime da Microsoft), não é distribuído pelo DockMax.

## O que falta para distribuir o binário a outras pessoas

O instalador atual é para uso do autor. Antes de entregá-lo a terceiros, as licenças MIT, Apache-2.0 e BSD pedem que os avisos de copyright acompanhem o programa: gerar um arquivo de avisos (por exemplo com `cargo-about` e `pnpm licenses`) e mostrá-lo no painel Sobre ou junto ao instalador. Isso não foi feito.

## Como refazer a verificação

```bash
pnpm licenses list --prod
cd src-tauri
cargo tree --locked -e normal --target x86_64-pc-windows-msvc --prefix none -f "{p}|{l}" | sort -u
```
