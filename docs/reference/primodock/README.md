# Referência: o PrimoDock observado

Observações do PrimoDock 0.7.0-beta (Windows) guardadas aqui para o futuro, enquanto ele está instalado e com a licença ativa na máquina do autor. Complementam a [spec 0004](../../specs/0004-paridade-primodock.md), que traz o inventário de funcionalidades e as fases de paridade; aqui ficam os **dados observados**, com mais detalhe do que a spec comporta.

## Arquivos desta pasta

| Arquivo | Conteúdo |
|---|---|
| [settings-e-dados.md](settings-e-dados.md) | Esquema do `settings.json`, `workspaces.json`, `session.json` e `icons.json`, com valores e padrões vistos |
| [pacotes-de-icones.md](pacotes-de-icones.md) | Formato do pacote de ícones (`pack.json`), as 11 categorias, o catálogo da galeria e os tons dos widgets |
| [comportamentos.md](comportamentos.md) | O que o PrimoDock faz com a barra do Windows, a bandeja, os monitores e os apps em tela cheia, lido do registro de execução |
| [instalacao.md](instalacao.md) | Como está instalado, de que é feito e quais as licenças de terceiros |
| [interface.md](interface.md) | A interface em uso: o dock, os menus, a bandeja, os espaços de trabalho e a janela de configurações, por capturas descritas (as imagens ficam fora do git) |

## Como foi levantado

Só leitura, sem mexer no PrimoDock nem no que ele guarda: arquivos de `%APPDATA%\PrimoDock`, a pasta de instalação em `%LOCALAPPDATA%\PrimoDock.App` e o registro de execução (`primodock.log`). Os levantamentos anteriores, por capturas de tela do painel de configurações, estão resumidos na spec 0004 §3.

## Limites (clean-room)

O DockMax é uma reimplementação a partir do comportamento observado, com decisão do autor de 2026-10-22 de não copiar código, ativos, marca nem a lógica de licença do PrimoDock. Por isso esta pasta:

- **Não tem** código, binários, ícones, imagens dos pacotes, miniaturas, textos de licença nem nada que identifique a licença, o dispositivo ou a conta (o arquivo de período de teste e o identificador do dispositivo no registro de execução foram deliberadamente ignorados).
- **Não tem** dados pessoais: caminhos, nomes de apps e de espaços de trabalho do autor viraram exemplos genéricos.
- **Tem** nomes de campos e valores de configuração, formatos de arquivo, números medidos e a descrição de comportamentos. São fatos observáveis de um produto instalado, para guiar o desenho do DockMax, não uma cópia dele.

## O que ainda falta

A interface em uso está em [interface.md](interface.md), incluindo os 15 temas e as quatro bordas no próprio dock. Ainda sem registro: a animação de abertura num app de verdade (Pulo, Pulso, Tremor), o mascote comendo depois de "Colocar comida" e o tour.
