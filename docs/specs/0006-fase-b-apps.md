# Spec 0006 — Fase B: apps no dock

- **Status:** rascunho em implementação (recorte B1)
- **Data:** 2026-10-23
- **Origem:** [spec 0004](0004-paridade-primodock.md) §3.4, §3.7c e §6 (fase B) e [ADR 0009](../adr/0009-dock-substitui-a-barra-de-tarefas.md)

## 1. O problema

Com o dock no lugar da barra do Windows (fase A e spike da barra), ele só tem Configurações, Sobre, a bandeja e os widgets. Não dá para abrir um programa a partir dele, que é o uso principal de uma barra de tarefas.

## 2. Recortes

| Recorte | Entrega | Toca em |
|---|---|---|
| **B1** | Catálogo dos programas instalados, fixar, desafixar, reordenar, abrir ao clicar e o ícone real de cada um | banco (migração 2), `windows/apps.rs`, `commands/apps.rs`, `Dock.tsx`, Configurações |
| **B2** | Formato dos ícones (Placas arredondadas, Mesmo tamanho, Como vieram), animação de abertura (Pulo, Pulso, Tremor), arrastar para reordenar e soltar um programa no dock para fixá-lo | `settings`, `Dock.tsx`, CSS |
| **B3** | Menu do app no dock: a Jump List do Windows (spike antes de prometer), "Remover do Dock", "Mostrar no Explorador" | `windows/` |
| **B4** | Pacotes de ícones, ícone por item e imagem personalizada | a definir |

B1 não depende de apps em execução (fase D): o dock ainda não sabe quais janelas estão abertas, então o indicador de app aberto fica para a fase D.

## 3. B1 em detalhe

### 3.1 A fonte dos programas: a pasta Aplicativos do shell

O catálogo é a pasta `shell:AppsFolder`, a mesma lista de "Todos os aplicativos" do menu Iniciar. Ela traz programas clássicos e aplicativos da Loja do mesmo jeito, cada um com nome, ícone e um **nome de análise** (*parsing name*) que o abre: o AUMID de um app da Loja (`Microsoft.WindowsCalculator_8wekyb3d8bbwe!App`), ou a pasta conhecida mais o caminho, ou o próprio caminho, de um programa clássico.

Spike (2026-10-23, nesta máquina): 197 entradas listadas em cerca de 1 s a frio (0,3 s depois); ícone de 96 px em 20 a 70 ms cada, com transparência correta; nenhuma entrada sem nome de análise. Programas clássicos e da Loja saem pela mesma API (`IShellItem`, `IShellItemImageFactory`), sem ler arquivos `.lnk` nem o registro.

- **Abrir:** `ShellExecuteW` em `shell:AppsFolder\<nome de análise>`, como o menu Iniciar faz.
- **Ícone:** `IShellItemImageFactory::GetImage` (somente ícone, 96 px), em PNG com alfa direto.
- **Validação do id** no Rust antes de qualquer uso: texto de uma linha, sem caractere de controle, sem espaço nas pontas, até 512 caracteres. O prefixo `shell:AppsFolder\` confina o nome ao conjunto de aplicativos: um caminho qualquer não resolve ali.

### 3.2 Dados

Migração 2 do banco (`0002_apps.sql`, o mesmo `dockmax.db` dos widgets), tabela `STRICT`:

| Coluna | Papel |
|---|---|
| `id` | UUID v7 do item, gerado no Rust; nunca reaproveitado |
| `app_id` | nome de análise; único (comparado sem diferenciar maiúsculas) |
| `name` | nome mostrado, lido do próprio shell ao fixar (o dock continua mostrando o programa se ele for desinstalado) |
| `position` | 0, contíguo; a interface recebe a lista já na ordem |
| `created_at` | milissegundos |

Limite de 40 apps. O modo de recuperação e as cópias de segurança do banco (ADR 0008) valem para a tabela nova sem mudança. Com o banco indisponível, a lista de apps é vazia e as Configurações mostram o aviso que já existe.

Os ícones **não** vão para o banco: são lidos do Windows quando o dock abre e guardados só em memória (um `data:` URL de PNG por app, em torno de 10 KB). Assim um programa que muda de ícone na atualização aparece certo na próxima abertura.

### 3.3 Comandos

Todos validam a entrada no Rust (arquitetura §2 e §8). O erro é só um código, sem caminho nem nome.

| Comando | O que faz |
|---|---|
| `list_apps` | a lista fixada e o estado do armazenamento (`AppsSnapshot`) |
| `list_catalog` | o catálogo, em ordem alfabética (assíncrono: roda fora da thread principal) |
| `app_icon(appId)` | o ícone como `data:` URL, ou `null` |
| `pin_app(appId)` | fixa no fim; o nome vem do shell, não da página. Códigos: `invalidApp`, `alreadyPinned`, `limitReached`, `unavailable`, `writeFailed` |
| `unpin_app(id)` | remove; `notFound` se o id não existe |
| `move_app(id, direction)` | uma posição para cima ou para baixo, como os widgets |
| `launch_app(id)` | abre o app fixado; `notFound` ou `launchFailed` |

Depois de cada mudança aceita, o evento `apps-changed` leva o `AppsSnapshot` a todas as janelas, como o `widgets-changed`.

### 3.4 No dock

A ordem passa a ser **apps, widgets, itens do sistema**, com um separador entre grupos que existem (`groups` do `layout.ts` ignora grupo vazio). Cada app é um botão do dock com o ícone real sobre a placa neutra do tema; o rótulo é o nome. Clicar abre o programa. O ícone ainda não chegou, ou não existe: a inicial do nome sobre a placa.

Sem apps o dock fica como é hoje.

### 3.5 Nas Configurações

Uma linha nova, **Apps do dock**, abre uma subvisão (padrão de "Itens do dock"):

- **Fixados:** lista na ordem do dock, cada linha com o ícone, o nome, subir, descer e remover.
- **Adicionar:** busca por nome sobre o catálogo (sem diferenciar maiúsculas nem acentos) e a lista; clicar fixa. Os já fixados aparecem marcados e não se fixam duas vezes.

### 3.6 Fora de B1

Reordenar arrastando, soltar um programa no dock, menus de contexto, indicador de app aberto, ativar ou minimizar janelas, e pacotes de ícones. Programas que não estão na pasta Aplicativos (um `.exe` solto, um atalho de área de trabalho) só entram em B2, com o soltar.

## 4. Testes

- **Rust:** domínio (limites, validação do id), repositório (fixar, duplicado sem diferenciar maiúsculas, remover renumera, mover nas pontas, limite), migração 2 sobre um banco da versão 1 (os widgets sobrevivem), comandos sem Tauri, e o guarda de capacidades.
- **TS:** o dock com apps (ordem dos grupos, ícone, inicial de reserva, clique abre), o painel (fixar, desafixar, mover, busca, erro) e o contrato de erros.
- **App real:** fixar alguns programas, ver os ícones no dock e medir a memória (meta do projeto). Abrir um programa a partir do dock só com o aviso e a autorização do autor.

## 5. Verificação

(preenchida ao fim da implementação)
