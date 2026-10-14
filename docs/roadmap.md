# Roadmap — DockMax

**Produto:** DockMax  
**Plataforma inicial:** Windows 11  
**Objetivo:** ferramenta de uso pessoal que também demonstre competências de desenvolvimento desktop e arquitetura.  
**Status:** proposta inicial; estimativas são indicativas.

## Visão do produto

DockMax será um dock personalizável para Windows, inspirado em aplicações de dock e widgets, mas com identidade própria. A interface principal reunirá atalhos e painéis de widgets. A integração com a barra de tarefas será evolutiva: o DockMax começará convivendo com a barra nativa e poderá incorporar capacidades de gerenciamento de janelas ao longo do tempo.

## Princípios de escopo

- Primeiro entregar um aplicativo local útil; não criar backend remoto sem necessidade.
- Separar widgets visuais de serviços nativos e integrações externas.
- Implementar widgets por prioridade e dependência técnica.
- Tratar recursos que dependem de hardware ou APIs externas como capacidades opcionais.
- Medir consumo de memória/CPU antes de otimizar prematuramente.
- Comercialização, suporte a macOS e sincronização em nuvem estão fora do escopo inicial.

## Fases

### Fase 0 — Preparação e validação técnica

**Estimativa:** 2–4 dias.

- [x] Criar repositório e configurar Git.
- [x] Instalar/verificar Node.js, Rust toolchain e dependências necessárias do Tauri no Windows.
- [x] Criar esqueleto Tauri 2 + React + TypeScript.
- [x] Confirmar build e execução local.
- [x] Configurar formatação, lint e testes.
- [x] Registrar versões e comandos de desenvolvimento no README.

**Critério de saída:** aplicação vazia abre localmente e pode ser compilada sem erros.

### Fase 1 — Casca do aplicativo e dock visual

**Estimativa:** 1–2 semanas.

- [ ] Implementar janela do dock e comportamento inicial.
- [ ] Criar componentes visuais do dock, ícones e painel.
- [ ] Definir tokens de tema com variáveis CSS.
- [ ] Permitir abrir/fechar painéis.
- [ ] Implementar configurações básicas de posição, tamanho e tema.
- [ ] Definir tratamento de erros e logs sem informações sensíveis.

**Critérios de aceitação**
- Dock abre e fecha sem falhas.
- Painéis podem ser abertos e fechados.
- Tema e preferências sobrevivem a reinicializações.
- Interface continua utilizável em uma escala de tela comum.
- Falhas de um painel não derrubam a janela principal.

### Fase 2 — Núcleo de widgets e persistência

**Estimativa:** 1–2 semanas.

- [ ] Definir o contrato de widget e o registro central.
- [ ] Implementar persistência local e migrações.
- [ ] Criar widgets Relógio, Progresso do tempo e Nota.
- [ ] Criar estrutura inicial para Atalho e Arquivos.
- [ ] Implementar validação de configurações.
- [ ] Adicionar testes unitários para regras de negócio.

**Critérios de aceitação**
- Instâncias de widgets têm IDs estáveis.
- Configurações inválidas são tratadas sem bloquear a inicialização.
- Notas persistem após reiniciar o aplicativo.
- A lógica de negócio pode ser testada sem renderizar a interface.

### Fase 3 — Atalhos, arquivos e prateleira

**Estimativa:** 1–2 semanas.

- [ ] Abrir aplicativos, arquivos, pastas e URLs com validação.
- [ ] Permitir fixar, remover e reordenar atalhos.
- [ ] Implementar widget Arquivos para locais escolhidos pelo usuário.
- [ ] Implementar Prateleira com drag-and-drop e referências a arquivos.
- [ ] Definir o que ocorre quando um arquivo de referência é movido ou removido.
- [ ] Restringir comandos nativos ao necessário.

**Critérios de aceitação**
- Atalhos válidos abrem o destino esperado.
- Erros de caminho são apresentados de forma compreensível.
- A Prateleira não duplica arquivos automaticamente.
- Nenhum comando arbitrário pode ser executado diretamente a partir de conteúdo não confiável.

### Fase 4 — Integração com o Windows

**Estimativa:** 2–4 semanas; pode variar significativamente.

- [ ] Implementar provedores de bateria e rede.
- [ ] Investigar enumeração, foco, ativação e minimização de janelas.
- [ ] Definir comportamento de janela em tela cheia e múltiplos monitores.
- [ ] Testar inicialização com o Windows.
- [ ] Avaliar limites de acesso a métricas de hardware.
- [ ] Adicionar testes e fallback quando uma API não estiver disponível.

**Critérios de aceitação**
- Dados indisponíveis são identificados como indisponíveis, não inventados.
- Erros de integração não encerram o aplicativo.
- O DockMax pode ser usado junto da barra de tarefas nativa.
- Comportamentos de foco e janela estão documentados.

### Fase 5 — Empacotamento e MVP

**Estimativa:** 1–2 semanas.

- [ ] Gerar instalador Windows.
- [ ] Testar instalação e desinstalação em ambiente limpo.
- [ ] Verificar persistência de configurações após atualização manual.
- [ ] Documentar limitações conhecidas.
- [ ] Criar README com imagens, arquitetura, funcionalidades e instruções.
- [ ] Criar uma demonstração curta do aplicativo.
- [ ] Verificar licença de dependências e atribuições necessárias.

**Critérios de aceitação do MVP**
- [ ] Instala e inicia em uma máquina Windows compatível.
- [ ] Dock e painéis funcionam sem falhas críticas conhecidas.
- [ ] Atalhos e preferências persistem.
- [ ] Notas são salvas e recuperadas.
- [ ] Temas são aplicados de forma consistente.
- [ ] Testes essenciais passam.
- [ ] Documentação explica como executar, testar e compilar.
- [ ] Limitações de gerenciamento de janelas estão explícitas.

## Backlog pós-MVP

### Produtividade
- [ ] Calendário
- [ ] Lembretes e notificações
- [ ] Área de transferência
- [ ] Beber água
- [ ] Melhorias na Prateleira e nos grupos de atalhos

### Sistema e mídia
- [ ] Atividade: CPU e memória; outros indicadores conforme viabilidade
- [ ] Música: sessão de mídia do Windows
- [ ] Ventoinhas e temperaturas: somente com provedor compatível
- [ ] Conta-gotas de cores

### Integrações
- [ ] GitHub
- [ ] Clima
- [ ] Uso do Claude: fontes separadas para Claude Code local, API e dados de assinatura/organização disponíveis

## Widgets planejados

1. Atalho
2. Arquivos
3. Prateleira
4. Relógio
5. Progresso do tempo
6. Calendário
7. Lembretes
8. Nota
9. Música
10. Bateria
11. Atividade
12. Ventoinhas
13. Rede
14. Conta-gotas
15. Área de transferência
16. Beber água
17. Uso do Claude
18. GitHub
19. Clima

**Nota:** a lista consolidada contém 19 itens. O widget “Atividade” deverá ter seu escopo definido antes da implementação (por exemplo, CPU/RAM ou atividade de aplicativos).

## Riscos e mitigação

| Risco | Impacto | Mitigação |
|---|---|---|
| Substituição completa da barra de tarefas | Alto | Manter convivência no MVP; evoluir por capacidades |
| APIs de hardware inconsistentes | Alto | Provedores substituíveis e estado “indisponível” |
| Métricas do Claude não acessíveis para conta individual | Alto | Identificar fonte e tipo de conta; não prometer limites não disponíveis |
| Consumo excessivo de recursos | Médio | Atualização centralizada e medição |
| Credenciais expostas | Alto | Armazenamento seguro e permissões mínimas |
| Escopo crescendo demais | Alto | Definir critérios de aceite por fase e adiar extras |

## Processo de trabalho com IA

Para cada tarefa:
1. Descrever objetivo e critérios de aceitação.
2. Pedir alternativas e riscos antes de codificar quando houver decisão arquitetural.
3. Implementar um incremento pequeno.
4. Executar testes e revisar o diff.
5. Atualizar documentação quando uma decisão mudar.
6. Criar commits atribuídos somente ao usuário, conforme a configuração de autoria desejada.

## Estimativa geral

Com dedicação de aproximadamente 10–15 horas por semana, um MVP básico pode levar cerca de 5–9 semanas. A parte de gerenciamento avançado de janelas pode estender o cronograma. As estimativas devem ser revisadas após a Fase 0.
