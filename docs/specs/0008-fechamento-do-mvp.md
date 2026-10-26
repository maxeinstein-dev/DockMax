# Spec 0008 — Fechamento do MVP: início com o Windows e instalador

O [escopo do MVP](../roadmap.md) é o que existe, com os ícones que os programas já têm. Esta spec cobre o que faltava para usar o DockMax no dia a dia no lugar da barra de tarefas e para instalá-lo: abrir com o Windows e o instalador. O resto da fase 5 (README, limitações, medição, licenças) está no [roadmap](../roadmap.md) e em [limitações](../limitacoes.md).

## 1. Iniciar com o Windows

Um dock que substitui a barra de tarefas precisa estar lá depois de entrar no Windows.

- **Configuração:** `launchAtLogin` em `settings.json` (nível do `Settings`, não do dock), desligada por padrão. Em Configurações, a chave "Iniciar com o Windows" ("Abre o DockMax ao entrar no Windows").
- **Como:** o Rust escreve a entrada `DockMax` na chave do usuário `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, com o caminho do programa entre aspas; desligar remove a entrada. É a mesma lista que a página "Aplicativos de inicialização" do Windows mostra e deixa o usuário desligar. Só a chave do usuário: não pede administrador e não toca em mais nada.
- **Quando:** ao mudar a chave, e uma vez por execução de uma build de **release** com a chave ligada (um programa que mudou de pasta numa atualização volta a ser achado). Builds de desenvolvimento (`debug`) nunca se registram, e a build do teste ponta a ponta (`feature e2e`) nunca toca na chave de quem roda os testes.
- **Falha:** a configuração fica como o usuário escolheu; o erro vai para o log (sem caminho); a próxima execução de release escreve a entrada de novo.
- **Desinstalar:** o desinstalador remove a entrada (e a cópia do vigia da barra, `dockmax-guard.exe`). Os dados do usuário ficam, a menos que ele peça para apagá-los.

## 2. O instalador

- **Formato:** NSIS (`.exe`), só ele (`bundle.targets = ["nsis"]`): o MSI exigiria baixar o WiX e não traz nada que o NSIS não traga aqui.
- **Por usuário** (`installMode = currentUser`): instala em `%LOCALAPPDATA%\DockMax`, sem pedir administrador, que o DockMax não usa. Em português (`PortugueseBR`).
- **Não é assinado:** o SmartScreen avisa; está nas [limitações](../limitacoes.md).
- **Tamanho:** cerca de 2,6 MiB.

## 3. Verificação

Ver "Medição de recursos" e "Verificação do instalador" no [roadmap](../roadmap.md), preenchidas com o que foi medido e testado.
