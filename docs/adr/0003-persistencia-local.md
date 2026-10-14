# ADR 0003 — Persistência local

- **Status:** Aceita como proposta inicial
- **Data:** 2026-10-14

## Contexto

O MVP é para uso pessoal e não exige conta, sincronização ou backend. Alguns dados são estruturados (notas, lembretes, instâncias de widgets); outros são configurações simples ou arquivos do usuário.

## Decisão

Usar SQLite para dados estruturados e migrações versionadas. Usar arquivos de configuração apenas quando forem mais adequados. Guardar credenciais em armazenamento seguro, nunca em JSON comum ou no código frontend.

## Consequências

- O aplicativo funciona offline para os recursos locais.
- Migrações e recuperação de dados precisam ser testadas.
- Não armazenar indefinidamente métricas de sistema sem caso de uso.
- A Prateleira guarda referências a arquivos por padrão, sem copiá-los automaticamente.
