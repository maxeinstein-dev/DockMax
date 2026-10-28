# Spec 0018: arrastar entre o dock e as pastas

**Status:** em andamento.

Pedido do autor: "ao arrastar o app do dock para a pasta, abrir a pasta dentro do app. Por exemplo, arrastar o VS Code para uma pasta no Explorer faz abrir o VS Code dentro dessa pasta. Assim como no terminal. O contrário também poderia ser válido: arrastar a pasta ou o arquivo para o app faz com que abra aquele arquivo com o app."

## 1. As duas direções

| Gesto | Resultado |
|---|---|
| Arrastar um app do dock e soltar sobre uma **pasta** no Explorer ou na área de trabalho | O app abre nessa pasta |
| Soltar sobre um **arquivo** no Explorer ou na área de trabalho | O app abre esse arquivo |
| Soltar no **vazio** de uma janela do Explorer | O app abre na pasta que a janela mostra (na aba visível) |
| Soltar no vazio da área de trabalho | O app abre na pasta Área de Trabalho |
| Soltar em qualquer outro lugar fora do dock | Nada acontece: o bloco volta para o lugar |
| Arrastar arquivos ou pastas e soltar **sobre um app** do dock (fixado ou aberto) | O app abre o que foi solto |
| Soltar um `.exe` ou atalho **entre** os blocos ou no vazio do dock | Fixa o app, como já faz (spec 0006 B2) |

Igual ao Dock do macOS: em cima de um app, abre com ele; entre os blocos, fixa.

## 2. Como o app abre

- **Arquivos ou pasta entregues ao app:** como quando se solta arquivos no atalho do app no Windows. O item do app na pasta Apps recebe a soltura e o Windows monta os argumentos certos, inclusive os do atalho (um perfil do Chrome, por exemplo) e os apps da Store que aceitam arquivos. Sem esse caminho, um app clássico abre com os caminhos como argumentos.
- **Terminais abrem na pasta, não com ela de argumento:** o Windows Terminal abre com `wt -d "<pasta>"`; o Prompt de Comando, o Windows PowerShell e o PowerShell 7 abrem com a pasta de trabalho na pasta. Um arquivo solto num terminal abre o terminal na pasta do arquivo.

## 3. No dock

- Enquanto o bloco é arrastado dentro do dock, reordena como hoje. Ao sair da janela do dock, o bloco volta ao lugar e o cursor vira o ícone do app; ao voltar, a reordenação continua.
- Com arquivos sendo arrastados sobre o dock, o app sob o cursor fica destacado como no hover.
- Nenhum arquivo é copiado ou movido: o dock só abre.

## 4. Fora do escopo

- Soltar sobre a árvore de pastas lateral do Explorer (vale a pasta da janela).
- Soltar num widget (Arquivos, Nota) ou num item do sistema.
- Outros gerenciadores de arquivos além do Explorer.
