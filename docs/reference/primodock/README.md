# Referência: o PrimoDock observado

Observações do PrimoDock 0.7.0-beta (Windows) guardadas aqui para o futuro, enquanto ele está instalado e com a licença ativa na máquina do autor. Complementam a [spec 0004](../../specs/0004-paridade-primodock.md), que traz o inventário de funcionalidades e as fases de paridade; aqui ficam os **dados observados**, com mais detalhe do que a spec comporta.

## Arquivos desta pasta

| Arquivo | Conteúdo |
|---|---|
| [settings-e-dados.md](settings-e-dados.md) | Esquema do `settings.json`, `workspaces.json`, `session.json` e `icons.json`, com valores e padrões vistos |
| [pacotes-de-icones.md](pacotes-de-icones.md) | Formato do pacote de ícones (`pack.json`), as 11 categorias, o catálogo da galeria e os tons dos widgets |
| [comportamentos.md](comportamentos.md) | O que o PrimoDock faz com a barra do Windows, a bandeja, os monitores e os apps em tela cheia, lido do registro de execução |
| [instalacao.md](instalacao.md) | Como está instalado, de que é feito e quais as licenças de terceiros |

## Como foi levantado

Só leitura, sem mexer no PrimoDock nem no que ele guarda: arquivos de `%APPDATA%\PrimoDock`, a pasta de instalação em `%LOCALAPPDATA%\PrimoDock.App` e o registro de execução (`primodock.log`). Os levantamentos anteriores, por capturas de tela do painel de configurações, estão resumidos na spec 0004 §3.

## Limites (clean-room)

O DockMax é uma reimplementação a partir do comportamento observado, com decisão do autor de 2026-10-22 de não copiar código, ativos, marca nem a lógica de licença do PrimoDock. Por isso esta pasta:

- **Não tem** código, binários, ícones, imagens dos pacotes, miniaturas, textos de licença nem nada que identifique a licença, o dispositivo ou a conta (o arquivo de período de teste e o identificador do dispositivo no registro de execução foram deliberadamente ignorados).
- **Não tem** dados pessoais: caminhos, nomes de apps e de espaços de trabalho do autor viraram exemplos genéricos.
- **Tem** nomes de campos e valores de configuração, formatos de arquivo, números medidos e a descrição de comportamentos. São fatos observáveis de um produto instalado, para guiar o desenho do DockMax, não uma cópia dele.

## O que ainda falta e depende de olhar a interface

Estes pontos não aparecem nos arquivos e pedem o PrimoDock aberto, com capturas de tela e uso do mouse (a combinar antes com o autor): o aspecto exato de cada tema no dock, as animações (pulo, pulso, tremor, ampliação), o mascote, o menu de cada item, a prévia de janela, a janela de configurações e o tour. As capturas de 2026-10-22 e 2026-10-23 já cobrem parte (spec 0004 §3.7a e §3.7b).
