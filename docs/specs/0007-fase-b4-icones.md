# Spec 0007 — Fase B4: pacotes de ícones

Continuação da [spec 0006](0006-fase-b-apps.md) (B4) e da [spec 0004](0004-paridade-primodock.md). A referência do que o PrimoDock faz está em [`docs/reference/primodock/pacotes-de-icones.md`](../reference/primodock/pacotes-de-icones.md); **nenhum ícone nem miniatura dele é usado**: o formato abaixo é do DockMax.

## 1. O problema

Os apps do dock mostram o ícone que o Windows dá a cada programa. O usuário quer vestir o dock com um pacote de ícones (todos os apps de uma vez), voltar ao original, e mais tarde trocar o ícone de um app só ou usar uma imagem própria.

## 2. Recortes

| Recorte | Entrega |
|---|---|
| **B4a** | O formato do pacote, a leitura segura da pasta de pacotes, a escolha de um pacote para todos os apps (Original ou um pacote) em Configurações, "Abrir pasta de pacotes" e "Recarregar pacotes", a busca por nome e o ícone de reserva |
| **B4b** | O menu do app ganha "Ícone ›": Original, um pacote, "Escolher imagem…" (um arquivo do disco, copiado para a pasta de dados) |
| **B4c** | Chaves semânticas (browser, terminal…) para um programa sem ícone próprio no pacote, e pacotes que acompanham o app |

Esta spec detalha o B4a; B4b e B4c ganham seção própria quando começarem.

## 3. B4a em detalhe

### 3.1 O pacote

Uma pasta em `<pasta de dados>\IconPacks\<id>\` com um `pack.json` e imagens PNG:

```json
{
  "name": "Cores lisas",
  "author": "Fulano",
  "version": "1.0.0",
  "description": "Placas lisas, uma cor por programa.",
  "accent": "#3A63D6",
  "icons": {
    "Calculadora": "apps/calculadora.png",
    "Bloco de notas": "apps/bloco-de-notas.png"
  },
  "fallback": "fallback.png",
  "dark": {
    "apps/calculadora.png": "escuro/calculadora.png"
  }
}
```

- `<id>` é o nome da pasta: minúsculas, dígitos e `-`, de 1 a 48 caracteres. Uma pasta com outro nome é ignorada.
- `name` é obrigatório (até 60 caracteres); `author`, `version`, `description` e `accent` (`#RRGGBB`) são opcionais, para o cartão em Configurações.
- `icons` liga o **nome de um programa** a um arquivo, relativo à pasta do pacote. A chave vale para o nome mostrado do programa ou para o nome do arquivo dele; maiúsculas, acentos, espaços e pontuação não contam (`Bloco de notas`, `bloco-de-notas` e `BLOCO DE NOTAS` são a mesma chave), e uma extensão `.exe` ou `.lnk` é ignorada.
- `fallback` é o ícone de um programa que nenhuma chave cobre. Sem ele, esse programa fica com o ícone original.
- `dark` (opcional, acrescentado em 2026-10-30) liga uma imagem do pacote à que o dock desenha no lugar dela **enquanto está escuro**: o tema Escuro, ou Sistema com o Windows escuro. Uma imagem sem par no `dark` vale nos dois temas. As duas pontas são caminhos com as mesmas regras de `icons`, e o mapa tem no máximo 2000 entradas. Trocou o tema, o dock pede os ícones de novo: cada ícone guardado tem o tema na chave.
- A busca é: nome mostrado, nome do arquivo (para um programa solto no dock), ícone de reserva, ícone original. A imagem achada é trocada pelo par dela no `dark` se o dock está escuro.

### 3.2 Segurança

A pasta é do usuário, mas o que vem nela pode ter vindo de um download: o app trata o conteúdo como entrada não confiável.

- `pack.json` até 256 KiB; no máximo 2000 entradas; chaves de até 100 caracteres.
- Cada caminho é relativo, com `/` ou `\`, sem `..`, sem unidade, sem `:` (fluxos do NTFS) e termina em `.png`.
- A pasta do pacote é resolvida para o caminho real e o arquivo também: um atalho ou link simbólico que leve para fora da pasta é recusado.
- Cada imagem tem até 512 KiB e precisa começar com a assinatura PNG e declarar um tamanho de 16 a 1024 px por lado (lido do cabeçalho, sem decodificar).
- Um pacote com erro é listado como inválido (com o motivo, sem caminhos) e não pode ser escolhido; os outros continuam. Uma imagem ruim de um pacote bom só deixa aquele programa no ícone original.
- O ícone chega à página como `data:image/png;base64`, como o do Windows: a página nunca recebe um caminho e não lê a pasta.

### 3.3 Escolha e dados

`dock.iconPack` em `settings.json`: o id do pacote, ou ausente para o original (`Option<String>`, validado como o id). Se o pacote escolhido sumiu da pasta, o dock mostra os ícones originais e a escolha continua gravada (voltando o pacote, volta o efeito); Configurações mostra o cartão como "indisponível".

### 3.4 Comandos

| Comando | Quem chama | O que faz |
|---|---|---|
| `list_icon_packs` | painel | lê a pasta (de novo) e devolve `{ id, name, author, version, description, accent, iconCount, covers, total, error }` por pacote; `covers` de `total` é quantos dos apps fixados ele veste. Se o que a pasta tem difere do que estava guardado, o dock lê os ícones de novo |
| `reload_icon_packs` | painel | o botão "Recarregar pacotes": o mesmo, e o dock lê os ícones de novo **sempre** (uma imagem pode ter sido trocada sem mudar o `pack.json`) |
| `open_icon_packs_folder` | painel | cria a pasta se preciso e a abre no Explorador |
| `app_icon` (existente) | dock e painel | passa a consultar o pacote escolhido antes do Windows |

Quando o dock deve ler os ícones de novo, o Rust esquece os que guardou e envia `icons-changed` a todas as janelas; o dock esquece os que recebeu e os pede outra vez.

### 3.5 No dock e em Configurações

- O ícone do pacote vale para o **dock** (tamanho 96 do `app_icon`); a lista de programas de Configurações continua mostrando o ícone original de cada programa, para o usuário reconhecer o que está escolhendo. O ícone entra no lugar do original no mesmo ladrilho; o formato (Placas arredondadas, Mesmo tamanho, Como vieram) continua valendo, e "Como vieram" é o certo para um pacote que já traz a placa.
- Em Configurações, na aba Aparência, a seção "Ícones dos apps" (abaixo de "Formato dos ícones") mostra um cartão por opção: Original e um por pacote (nome, autor, "Veste 5 de 9 apps"), os botões "Abrir pasta de pacotes" e "Recarregar pacotes".

### 3.6 Fora de B4a

Ícone por app, imagem própria, chaves semânticas, galeria e download de pacotes, e pacote embutido (§2).

## 4. Verificado

No app real (build de debug), com um pacote de exemplo e um pacote quebrado na pasta, três programas fixados: a lista traz o pacote (cobre 3 de 3 apps) e o quebrado com o motivo; escolher o pacote troca os três ícones do dock; um pacote que sumiu deixa os ícones originais; "Recarregar" depois de trocar uma imagem do pacote por uma inválida devolve o ícone original só daquele programa; voltar a Original devolve os três. A seção em Configurações tem testes de componente; não foi vista no painel real.

## 5. Testes

- Rust: a normalização das chaves; o manifesto (campos obrigatórios, limites, caminhos recusados: `..`, absoluto, `C:`, `a:b`, não `.png`); a busca (nome, arquivo, reserva); a leitura de uma pasta real (pacote bom, pacote inválido, link para fora, PNG falso, PNG grande demais); `iconPack` em `settings.json` (padrão, id inválido).
- Página: o cartão de cada pacote e do original, a escolha grava `iconPack`, o dock esquece os ícones ao trocar de pacote.
- App real: um pacote de exemplo na pasta, escolhido; os apps mudam de ícone; trocar para Original volta; "Recarregar" acha um pacote colocado depois.
