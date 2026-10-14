# Arquitetura — DockMax

## 1. Contexto e objetivos

DockMax é uma aplicação desktop local para Windows, com interface em React/TypeScript e integração nativa via Tauri 2/Rust.

Objetivos arquiteturais:

- Separar interface, regras de aplicação, persistência e integrações nativas.
- Facilitar a implementação incremental de widgets.
- Testar regras sem depender de APIs reais do Windows.
- Minimizar permissões e exposição de dados.
- Evitar infraestrutura remota desnecessária.

Não objetivos iniciais:

- Suporte multiplataforma.
- Backend web ou sincronização em nuvem.
- Substituição completa da barra de tarefas no MVP.
- Sistema de plugins de terceiros carregados dinamicamente.
- Implementação de todos os widgets na primeira versão.

## 2. Visão de alto nível

```text
React UI
  ├── Dock
  ├── Widget Panels
  ├── Settings / Themes
  └── Widget Components
          │
          ▼
Frontend Application Services
  ├── Widget Registry
  ├── Settings
  ├── Commands / Typed API
  └── Event Subscriptions
          │ Tauri IPC
          ▼
Rust Application Layer
  ├── Commands (thin adapters)
  ├── Domain / Use Cases
  ├── Ports (traits)
  ├── Persistence
  └── Adapters
       ├── Windows APIs
       ├── Hardware providers
       └── External services
```

### Regra de dependência

A interface depende dos contratos públicos, não da implementação interna de Windows ou SQLite. Os comandos Tauri validam entrada, chamam casos de uso e convertem erros em respostas seguras. A lógica de negócio não deve ficar dentro de handlers de comandos.

## 3. Stack

- React + TypeScript + Vite.
- Tauri 2.
- Rust.
- SQLite para dados estruturados.
- CSS Modules ou CSS com tokens/variáveis bem definidos.
- Vitest e React Testing Library.
- Testes unitários Rust.
- Testes end-to-end com ferramenta compatível com o setup Tauri.

Fixar versões reais na implementação e verificar requisitos atuais do Tauri, Rust e plugins antes de instalar.

## 4. Organização de diretórios sugerida

```text
src/
  app/
  components/
    dock/
    panels/
    settings/
    ui/
  widgets/
    registry.ts
    shortcuts/
    files/
    shelf/
    clock/
    time-progress/
    calendar/
    reminders/
    notes/
    music/
    battery/
    activity/
    fans/
    network/
    eyedropper/
    clipboard/
    drink-water/
    claude-usage/
    github/
    weather/
  core/
    widget-contracts/
    events/
    settings/
    errors/
  services/
    desktop/
    persistence/
    integrations/
  themes/
src-tauri/
  capabilities/
  migrations/
  src/
    commands/
    domain/
    ports/
    adapters/
      windows/
      hardware/
      integrations/
    persistence/
    lib.rs
    main.rs
tests/
  integration/
  e2e/
docs/
  adr/
```

Criar diretórios quando houver código real para colocar neles; não criar módulos vazios apenas para antecipar uma arquitetura futura.

## 5. Contrato de widgets

Um widget possui:
- `id` estável do tipo de widget;
- nome, descrição e categoria;
- configuração padrão validável;
- tamanho inicial sugerido;
- capacidades requeridas;
- componente de apresentação;
- identificador único por instância;
- política explícita para persistência e ciclo de vida, quando necessário.

Exemplo conceitual em TypeScript:

```ts
export type WidgetCategory =
  | "productivity"
  | "time"
  | "system"
  | "media"
  | "developer"
  | "utilities";

export interface WidgetDefinition<TConfig = unknown> {
  id: string;
  name: string;
  description: string;
  category: WidgetCategory;
  defaultSize: { width: number; height: number };
  defaultConfig: TConfig;
  capabilities: string[];
}

export interface WidgetContext {
  widgetInstanceId: string;
  publishEvent(event: string, payload?: unknown): void;
}

export interface WidgetModule<TConfig = unknown> {
  definition: WidgetDefinition<TConfig>;
  Component: React.ComponentType<{
    config: TConfig;
    context: WidgetContext;
  }>;
  validateConfig(value: unknown): value is TConfig;
}
```

Este contrato é ilustrativo. Ajustá-lo depois de implementar os primeiros widgets; evitar um sistema genérico complexo antes de existir necessidade real.

### Ciclo de vida

Widgets puramente visuais não precisam de métodos artificiais de inicialização e encerramento. Widgets que assinam eventos ou mantêm recursos devem usar hooks/serviços com cleanup explícito. Serviços de sistema de uso compartilhado devem ser centralizados para evitar polling duplicado.

### Erros

- Configuração inválida: validar, recuperar valor padrão quando seguro e registrar aviso sem dados sensíveis.
- Serviço indisponível: apresentar estado de indisponibilidade.
- Falha de um widget: isolar a falha; não encerrar a aplicação.
- Permissão negada: explicar qual capacidade está indisponível.
- Integração externa sem rede: manter UI responsiva e indicar dados desatualizados.

## 6. Persistência

SQLite para entidades estruturadas:
- `widget_instances`
- `dock_settings` ou tabela de configurações versionada
- `notes`
- `reminders`
- futuras tabelas específicas quando necessárias

Cada instância deve ter ID estável, tipo de widget, estado habilitado, posição e configuração validada/versionada.

Não persistir métricas de CPU/RAM/rede indefinidamente. Começar com snapshots em memória; adicionar histórico apenas se houver um caso de uso concreto.

Configurações globais simples podem ficar em um arquivo de configuração. Não armazenar tokens ou segredos em texto puro no banco ou em JSON. Usar armazenamento seguro compatível com o ambiente.

## 7. Contratos nativos e serviços

Criar ports/traits Rust para capacidades que podem ter mais de uma implementação ou precisam ser mockadas:

- `AppLauncher`: abrir aplicações, arquivos, pastas e URLs.
- `WindowManager`: listar janelas elegíveis, ativar e minimizar quando tecnicamente suportado.
- `SystemMetricsProvider`: snapshots de bateria, CPU, memória e rede.
- `FanSensorProvider`: métricas de ventoinhas, com disponibilidade opcional.
- `ClipboardProvider`: acesso ao clipboard dentro das limitações do Windows e das permissões.
- `MediaSessionProvider`: metadados e controles de mídia compatíveis.
- `NotificationService`: notificações locais.
- `ExternalIntegration`: abstração por integração externa quando útil.

Não criar abstrações para toda função simples. Criar uma port quando ela isola uma API externa, permite teste, ou representa uma capacidade de domínio relevante.

## 8. Segurança

- Aplicar capacidades e permissões Tauri mínimas.
- Não habilitar acesso genérico a shell sem necessidade.
- Não aceitar comandos arbitrários da UI.
- Validar caminhos, URLs e payloads no lado nativo.
- Armazenar credenciais fora do frontend e protegê-las com mecanismo apropriado.
- Evitar registrar conteúdo do clipboard, notas, tokens e dados privados.
- Para GitHub/Claude/clima, usar autenticação oficial e respeitar limites e termos dos serviços.
- Não baixar nem executar código de widgets de fontes externas.

## 9. Desempenho e ciclo de vida

- Centralizar polling para recursos compartilhados.
- Permitir intervalo configurável por provedor.
- Pausar ou reduzir atualizações de widgets que não precisam de atualização constante.
- Não executar chamadas nativas em cada renderização.
- Cancelar listeners e timers quando o recurso deixa de ser usado.
- Medir consumo real antes de criar otimizações complexas.

## 10. Dock e gerenciamento de janelas

O MVP convive com a barra de tarefas nativa. Evolução posterior:
1. Abrir aplicativos e destinos.
2. Enumerar janelas elegíveis.
3. Ativar e minimizar janelas.
4. Representar estado de aplicativos abertos.
5. Lidar com múltiplos monitores e escalas.
6. Investigar bandeja do sistema e outros comportamentos avançados.

Comportamento em tela cheia, foco, janela sempre no topo e ocultação automática precisam ser testados no Windows real. Não presumir que uma janela Tauri comum substitua as capacidades do shell do Windows.

## 11. Widget de uso do Claude

Tratar fontes como produtos de dados diferentes:
- Claude Code local: somente métricas encontradas em fontes locais suportadas.
- API: uso/custos se o usuário configurar credenciais válidas.
- Assinatura/organização: apenas dados expostos oficialmente para o tipo de conta.
- Analytics organizacional: respeitar requisitos de plano e permissões administrativas.

Exibir origem, horário de atualização e distinção entre dado oficial, estimativa e indisponibilidade. Nunca inferir limites de assinatura sem fonte confiável.

## 12. Testes

- Unitários TypeScript: validação de configuração, estado, formatação e lógica dos widgets.
- Componentes: estados normal, carregando, erro e indisponível.
- Unitários Rust: regras de domínio e conversão de erros.
- Integração: persistência/migrações e comandos com adaptadores simulados quando possível.
- E2E: abrir dock, mudar tema, criar nota, reiniciar e verificar persistência.
- Manual: monitores, escala, tela cheia, permissões, instalação e inicialização automática.

## 13. Observabilidade

Logs locais com níveis e contexto técnico, sem conteúdo sensível. Erros de serviços devem identificar a capacidade afetada. Não enviar telemetria remota no MVP.

## 14. Critérios de evolução arquitetural

Antes de adicionar uma abstração, responder:
1. Qual problema real ela resolve?
2. Existe mais de uma implementação, ou precisamos de mock para testar?
3. Ela reduz acoplamento ou só aumenta indireção?
4. O custo de manutenção é menor que o benefício?

Revisar este documento quando decisões reais divergirem do plano e registrar decisões importantes em ADRs.
