# ADR 0012 — Contas Google: WinHTTP, cofre do Windows e login pelo navegador

**Status:** aceito (2026-10-28).
**Contexto:** [spec 0017](../specs/0017-fase-f-widgets.md) §6c, F2a: o Calendário lê as agendas e as tarefas das contas Google do autor, a dele, a da Alfama e a da noiva, só para leitura.

## Contexto

Até aqui o DockMax não falava com nenhum serviço na internet. O F2a precisa de três coisas novas:

1. **HTTPS** para as APIs do Google (token, Agenda, Tasks).
2. **Um lugar seguro para guardar** os tokens de cada conta e o cliente OAuth que o autor cria no Google Cloud.
3. **O login**, sem que o DockMax veja a senha.

O orçamento de memória do roadmap continua valendo ([ADR 0010](0010-gpu-no-processo-do-navegador.md)). Cada dependência nova pesa no binário e, às vezes, na memória.

## Decisão

1. **HTTP pelo WinHTTP do Windows** (`windows/http.rs`), não por um crate como `reqwest` ou `ureq`.
   - O TLS fica com o sistema, que tem os certificados atualizados e o proxy do Windows.
   - Não entra nenhuma biblioteca de rede nem de TLS no binário.
   - As chamadas são poucas e simples (GET e POST de formulário com JSON de volta), e o WinHTTP já cobre isso.
2. **Segredos no cofre de credenciais do Windows** (`windows/credentials.rs`, `CredWriteW`):
   - o cliente OAuth fica em `DockMax/google-client`, com o ID e o segredo;
   - cada conta conectada fica em `DockMax/google:<e-mail>`, com o refresh token.
   - Nada disso vai para o `settings.json`, o banco ou os registros (logs). O cofre é cifrado pelo Windows para o usuário logado.
   - A lista de contas é a lista dessas credenciais, e o token de acesso (1 h) fica só na memória.
3. **Login de app instalado** (RFC 8252): o DockMax abre o navegador padrão na tela de login do Google, com PKCE (S256) e um `state` aleatório, e escuta a volta em `http://127.0.0.1:<porta livre>`.
   - O DockMax nunca vê a senha.
   - A volta só é aceita com o mesmo `state`, e a escuta fecha depois de 5 minutos.
   - O PKCE (SHA-256) e os números aleatórios vêm do BCrypt do Windows.
4. **Cliente OAuth do próprio autor:** o DockMax não traz um cliente embutido. O autor cria o projeto no Google Cloud (em produção e sem verificação, decisão da §6c) e cola o ID e o segredo em Configurações › Contas. Assim o segredo nunca vai para o repositório.
5. **Escopos só de leitura:** `calendar.readonly` e `tasks.readonly`. O e-mail da conta vem da agenda principal (`calendarList`, `primary`), sem pedir `email` nem `openid`.
6. **Desconectar** revoga o token no Google (`/revoke`) e apaga a credencial. Se a revogação falhar (sem internet), a credencial é apagada mesmo assim, e o registro (log) diz que a revogação não foi feita.

## Consequências

- Nenhuma dependência nova no `Cargo.toml`, só recursos do crate `windows` que já é usado: `Win32_Networking_WinHttp`, `Win32_Security_Credentials` e `Win32_Security_Cryptography`.
- O DockMax passa a fazer requisições na internet, só para `accounts.google.com`, `oauth2.googleapis.com` e `www.googleapis.com`, e só com contas conectadas. Sem contas, nada sai da máquina.
- As regras puras ficam em `domain/google.rs` e têm testes: o PKCE com o vetor da RFC 7636, a URL de login, a leitura da volta e as respostas de token.
- Um cliente OAuth trocado, apagado ou revogado no Google faz o refresh falhar. A conta aparece como "reconectar" em Configurações, sem travar o dock.
