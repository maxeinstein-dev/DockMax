# Política de privacidade do DockMax

Última atualização: 28 de outubro de 2026.

O DockMax é um dock para o Windows 11 feito por Maxsuel Einstein para uso pessoal. Esta página diz o que o DockMax faz com os dados das contas Google que você conecta a ele.

## O que o DockMax acessa

Quando você conecta uma conta Google em Configurações › Contas, o DockMax pede três permissões:

- **Ver as suas agendas e a lista delas** (`calendar.readonly`), para mostrar as próximas reuniões no widget Calendário.
- **Ver e editar eventos** (`calendar.events`), para criar uma reunião pelo dock quando você pedir. Nada é criado ou alterado sem a sua confirmação.
- **Ver e editar as suas tarefas** (`tasks`), para mostrar as tarefas do dia e marcar uma como feita quando você pedir.

O DockMax não pede acesso ao seu e-mail, aos seus contatos, ao seu Drive nem a nenhum outro dado da conta.

## Onde os dados ficam

- **Tudo fica no seu computador.** O DockMax não tem servidor e não manda os seus dados para nenhum lugar além dos próprios serviços do Google.
- Os eventos e as tarefas ficam **só na memória** enquanto o DockMax está aberto. Nada disso é gravado em disco.
- O token que mantém a conta conectada fica no **cofre de credenciais do Windows**, cifrado para o seu usuário. A sua senha nunca passa pelo DockMax: o login acontece no seu navegador, na página do Google.
- Os registros de funcionamento (logs) do DockMax nunca contêm tokens, eventos, tarefas, nomes de agenda nem endereços de e-mail.

## Compartilhamento

O DockMax não vende, não compartilha e não transfere os seus dados para ninguém. Não há publicidade, análise de uso nem rastreamento.

O uso das informações recebidas das APIs do Google segue a [Política de dados do usuário dos serviços de API do Google](https://developers.google.com/terms/api-services-user-data-policy), incluindo os requisitos de uso limitado.

## Como desconectar

Em Configurações › Contas, use **Desconectar**: o DockMax revoga o acesso no Google e apaga o token do cofre do Windows. Você também pode revogar o acesso a qualquer momento em [myaccount.google.com/permissions](https://myaccount.google.com/permissions).

## Contato

Dúvidas sobre esta política: [maxsueleinstein.dev@gmail.com](mailto:maxsueleinstein.dev@gmail.com) ou uma issue em [github.com/maxeinstein-dev/DockMax](https://github.com/maxeinstein-dev/DockMax/issues).

Esta política também está publicada em [maxsueleinstein.dev/dockmax/privacidade](https://maxsueleinstein.dev/dockmax/privacidade/).
