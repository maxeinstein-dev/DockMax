# Instalação e números do PrimoDock

## Como está instalado

`%LOCALAPPDATA%\PrimoDock.App`, instalado e atualizado pelo **Velopack**:

```
PrimoDock.App\
  PrimoDock.exe          lançador do Velopack (1,9 MB)
  Update.exe             atualizador do Velopack (5,3 MB)
  current\
    PrimoDock.exe        o app, num executável único (208 MB)
    THIRD-PARTY-NOTICES.md
    sq.version           manifesto do pacote (nuspec)
  packages\
    PrimoDock.App-0.7.0-beta-full.nupkg   pacote completo da versão (86 MB)
```

- **Manifesto** (`sq.version`): id `PrimoDock.App`, versão `0.7.0-beta`, canal `win-x64`, atalhos na área de trabalho e na raiz do menu Iniciar, AUMID do atalho `velopack.PrimoDock.App`.
- **Atualização**: a cada abertura, num servidor da própria empresa (canal `windows/win-x64`). Sem rede, segue funcionando.
- **Executável**: .NET com WPF publicado como arquivo único autocontido (por isso os 208 MB); o nome interno é `PrimoDock.dll`. Metadados: produto e empresa "PrimoDock", sem copyright declarado.

## De que é feito (avisos de terceiros)

| Biblioteca | Para quê, segundo o próprio aviso | Licença |
|---|---|---|
| ManagedShell (Cairo) | hospeda a área de notificação quando a barra do Windows está oculta | Apache 2.0 |
| Velopack | instala e atualiza | MIT |
| Bouncy Castle | confere a assinatura Ed25519 dos recibos de licença | MIT |

Mais o que o .NET traz. A spec 0004 §1 registra também SQLite entre as dependências, visto no executável.

## Processos e memória (2026-10-25, máquina do autor)

O PrimoDock roda em **dois processos**: o dock e um **vigia** que é filho dele, aberto com `--watchdog <pid do dock>`.

| Processo | Memória privada | Conjunto de trabalho | Threads | Handles |
|---|---|---|---|---|
| dock | 167,1 MB | 262,7 MB | 20 | 1169 |
| vigia (`--watchdog`) | 11,7 MB | 41,6 MB | 6 | 285 |
| **total** | **178,8 MB** | | | |

Com dois monitores, um dock em cada, três espaços de trabalho, dois widgets e uns 15 apps fixados, minutos depois de abrir.

**Comparação com o DockMax** (mesma máquina, mesma medida de memória privada somando o processo e os filhos): 177 MB parado, 218 MB com quatro apps fixados e o catálogo lido (spec 0006 §8). Um dock só, sem espaços de trabalho, mas com o WebView2. Estão na mesma faixa; o PrimoDock faz mais com a mesma memória.

Diferença de desenho que vale notar: o vigia do PrimoDock é **filho** do dock. O do DockMax é iniciado **fora** da árvore de processos de propósito (ADR 0009), porque um vigia filho morre junto quando se mata a árvore do processo pelo Gerenciador de Tarefas, e a barra do Windows ficava oculta. Vale testar se o PrimoDock tem esse problema.
