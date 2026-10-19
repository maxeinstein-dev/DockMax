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
- Medir memória e CPU em repouso em todo PR que mude a interface e comparar com a linha de base. Orçamento até o MVP: memória privada em repouso (processo + WebView2) no máximo 20% acima da linha de base da Fase 1 (182 MB), ou seja, cerca de 220 MB.
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

- [x] Implementar janela do dock e comportamento inicial.
- [x] Criar componentes visuais do dock, ícones e painel.
- [x] Definir tokens de tema com variáveis CSS.
- [x] Permitir abrir/fechar painéis.
- [x] Implementar configurações básicas de posição, tamanho e tema.
- [x] Definir tratamento de erros e logs sem informações sensíveis.

**Critérios de aceitação**
- Dock abre e fecha sem falhas.
- Painéis podem ser abertos e fechados.
- Tema e preferências sobrevivem a reinicializações.
- Interface continua utilizável em uma escala de tela comum.
- Falhas de um painel não derrubam a janela principal.

Critérios verificados em 2026-10-15 (spec 0002, verificações 1–21, e teste manual do autor no build de release).

### Fase 2 — Núcleo de widgets e persistência

**Estimativa:** 1–2 semanas.

- [x] Definir o contrato de widget e o registro central.
- [x] Implementar persistência local e migrações.
- [x] Criar widgets Relógio, Progresso do tempo e Nota.
- [x] Criar estrutura inicial para Atalho e Arquivos.
- [x] Implementar validação de configurações.
- [x] Adicionar testes unitários para regras de negócio.

**Critérios de aceitação**
- Instâncias de widgets têm IDs estáveis.
- Configurações inválidas são tratadas sem bloquear a inicialização.
- Notas persistem após reiniciar o aplicativo.
- A lógica de negócio pode ser testada sem renderizar a interface.

Revisão de 2026-10-19 (fim da Fase 2): a direção foi mantida e o roadmap ganhou orçamento de memória, a divisão da Fase 3 por valor e risco, o spike da janela única na Fase 4 e a poda dos widgets de baixo valor. Detalhes nas specs das fases.

### Fase 3 — Atalhos, arquivos e prateleira

**Estimativa:** 1–2 semanas. Dividida em três partes, por valor no uso diário e por risco; cada parte é um PR.

**3a — Atalhos** (maior valor, menor risco)
- [ ] Esconder o dock sobre aplicativos em tela cheia (adiantado da Fase 4: é o que mais fere "nunca atrapalhar" no dia a dia, e a detecção da janela em primeiro plano já existe no código nativo).
- [ ] Abrir aplicativos, arquivos, pastas e URLs com validação (`opener` com caminhos e URLs conferidos no Rust).
- [ ] Permitir fixar, remover e reordenar atalhos.
- [ ] Restringir comandos nativos ao necessário.
- [ ] Link do repositório no painel Sobre abrindo o navegador (hoje é texto selecionável).

**3b — Arquivos**
- [ ] Implementar widget Arquivos para locais escolhidos pelo usuário.
- [ ] Definir o que ocorre quando um arquivo de referência é movido ou removido.

**3c — Prateleira** (só depois da validação técnica)
- [ ] Validação técnica: soltar arquivos (drag-and-drop) numa janela cujo click-through é alternado pelo Rust (ADR 0006). Alternativa já decidida antes de começar, como na Fase 1.
- [ ] Implementar Prateleira com drag-and-drop e referências a arquivos.

**Critérios de aceitação**
- Atalhos válidos abrem o destino esperado.
- Erros de caminho são apresentados de forma compreensível.
- A Prateleira não duplica arquivos automaticamente.
- Nenhum comando arbitrário pode ser executado diretamente a partir de conteúdo não confiável.

### Fase 4 — Integração com o Windows

**Estimativa:** 2–4 semanas; pode variar significativamente.

- [ ] Spike medido da janela única: dock e painel na mesma janela transparente, usando o click-through dinâmico já provado (ADR 0006, V1), para cortar um renderizador do WebView2. Decidir por ADR com a medição de memória, foco e blur.
- [ ] Orçamento de CPU em repouso, medido junto com a memória.
- [ ] Implementar provedores de bateria e rede.
- [ ] Investigar enumeração, foco, ativação e minimização de janelas.
- [ ] Definir comportamento em múltiplos monitores (tela cheia já tratada na 3a).
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
- [ ] Verificar persistência de configurações e do banco após atualização manual, com uma migração real (v1 → v2) e o backup antes dela.
- [ ] Política de retenção dos arquivos `bak-v*` e `.corrompido-*` na pasta de dados.
- [ ] Documentar limitações conhecidas (inclusive: leitor de tela nunca testado).
- [ ] README com "onde ficam seus dados" (`%APPDATA%\dev.maxsueleinstein.dockmax`).
- [ ] Criar README com imagens, arquitetura, funcionalidades e instruções.
- [ ] Criar uma demonstração curta do aplicativo.
- [ ] Verificar licença de dependências e atribuições necessárias.

**Critérios de aceitação do MVP**
- [ ] Instala e inicia em uma máquina Windows compatível.
- [ ] Dock e painéis funcionam sem falhas críticas conhecidas.
- [ ] Atalhos e preferências persistem.
- [ ] Notas são salvas e recuperadas.
- [ ] Temas são aplicados de forma consistente.
- [ ] Memória privada em repouso dentro do orçamento (cerca de 220 MB) e CPU em repouso abaixo de 1%.
- [ ] Testes essenciais passam.
- [ ] Documentação explica como executar, testar e compilar.
- [ ] Limitações de gerenciamento de janelas estão explícitas.

## Backlog pós-MVP

### Produtividade
- [ ] Calendário
- [ ] Lembretes e notificações
- [ ] Área de transferência
- [ ] Melhorias na Prateleira e nos grupos de atalhos

### Sistema e mídia
- [ ] Atividade: CPU e memória; outros indicadores conforme viabilidade. Definir o escopo antes de qualquer PR.
- [ ] Música: sessão de mídia do Windows

### Só se sobrar (valor baixo para o custo; cada widget custou cerca de 2 mil linhas com testes na Fase 2)
- [ ] Ventoinhas e temperaturas: exige driver de sensores, com risco de segurança
- [ ] Conta-gotas de cores
- [ ] Beber água
- [ ] Uso do Claude: depende de dados que a conta pode não expor

### Personalização do dock
- [ ] Cor de destaque escolhida pelo usuário (o token `--accent` já existe)
- [ ] Ponto indicador personalizável: cor, forma e visibilidade

### Integrações
- [ ] GitHub
- [ ] Clima

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
| Memória do WebView2 (duas janelas = dois renderizadores; 182 MB na Fase 1, 280 MB no PR 2 da Fase 2) | Alto | Orçamento por fase, medição em todo PR de interface, spike da janela única na Fase 4 |
| Drag-and-drop numa janela com click-through alternado pelo Rust | Médio | Validação técnica antes da Prateleira (3c), com alternativa decidida |
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
7. Classificar cada PR como caro ou barato no plano: PR barato tem um revisor e dispensa a segunda rodada se não houver achado bloqueante; usar menos lanes quando as peças são pequenas.
8. Antes do teste manual do usuário, capturar o build real via CDP nos dois temas (pega contraste e controle faltando antes do teste).
9. Medir memória e CPU em repouso em todo PR que mude a interface.
10. Versionar por fase (v0.2.0 no fim da Fase 2), para o painel Sobre e o instalador nascerem alinhados.

## Estimativa geral

Com dedicação de aproximadamente 10–15 horas por semana, um MVP básico pode levar cerca de 5–9 semanas. A parte de gerenciamento avançado de janelas pode estender o cronograma. As estimativas devem ser revisadas após a Fase 0.
