# Todo Calendar — convenções

- Aplicativo desktop offline: Tauri 2, Rust, React, TypeScript estrito e SQLite pelo plugin SQL. Nenhum backend HTTP ou serviço remoto.
- Interface, validações, documentação e notificações em pt-BR; relógio de 24 horas e fuso local.
- UI em `src/components`; regras puras em `src/features`; persistência exclusivamente em `src/services`; estado de interface em Zustand.
- Componentes shadcn/ui pertencem ao projeto em `src/components/ui`. Manter acessibilidade, foco visível e suporte a teclado.
- Não persistir dados de calendário em localStorage nem substituir SQLite por mocks no aplicativo. Navegador sem Tauri deve informar a indisponibilidade do armazenamento.
- SQL parametrizado. Migrations incrementais em `src-tauri/migrations`; nunca alterar uma migration já distribuída. Foreign keys em todas as conexões (SQLx habilita por padrão; verificar ao conectar).
- Alterações relacionadas devem usar uma transação nativa com conexão dedicada. O pool do plugin SQL não garante que chamadas JS separadas de BEGIN/COMMIT usem a mesma conexão.
- Datas de calendário são datas civis locais, sem conversão implícita para UTC; instantes de entrega e auditoria são UTC. Eventos de dia inteiro usam término exclusivo. Preservar âncora e fuso das séries.
- Recorrências devem ser expandidas por intervalo, com identificador estável série + início original; persistir apenas exceções. UI e agendador devem consumir o mesmo motor nativo.
- Lembretes, bandeja e autostart pertencem ao Rust; autostart desativado por padrão. Não solicitar permissões de plugins antes de implementar a funcionalidade.
- Não marcar fases como validadas sem executar seus testes. Registrar bloqueios ambientais em `IMPLEMENTATION_PLAN.md`.
- Comandos: `npm.cmd run typecheck`, `npm.cmd test`, `npm.cmd run test:database`, `npm.cmd run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `npm.cmd run tauri dev`.
- O usuário autorizou concluir as Fases 3, 4, 5 e 6 em sequência, sem pedir consentimento entre fases. Executar autonomamente e registrar validações e limitações reais.
- O núcleo testável em `src-tauri/calendar-core` compartilha o pool SQLite do plugin no desktop; não criar um segundo armazenamento. Adaptadores de fuso, notificações e sistema ficam no crate Tauri.
- Preservar duração exata e metadados UTC dos horários ambíguos; a identidade da ocorrência continua sendo a âncora civil original. Registrar cada migration também no núcleo e no inicializador Tauri.
- Verificação adicional: `npm.cmd run test:native:core`. Em ambientes que bloqueiam subprocessos, `npm.cmd run test:restricted` e `npm.cmd run build:validation` verificam frontend sem equivaler a instalador ou homologação desktop.
