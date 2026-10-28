# Manus Calendar

Nas visões mensal e semanais, arraste um evento ou tarefa para mudar sua data ou horário. Em séries recorrentes, essa ação move somente a ocorrência arrastada. Para editar pelo teclado ou alterar toda a série, use o formulário de edição. A semana em grade exibe também o dia do mês em cada cabeçalho.

Em **Configurações → Aparência**, escolha o tamanho dos textos dos eventos (10–24 px) e combine negrito, itálico e sublinhado. Essas preferências ficam salvas no SQLite local.

Calendário pessoal offline para Windows 10 e Windows 11, feito com Tauri 2, Rust, React e TypeScript. Tarefas e eventos compartilham as visões Semana, Mês e Ano. A interface usa português brasileiro, relógio de 24 horas, fontes locais e SQLite por usuário. Não há conta, backend HTTP, telemetria ou atualização automática remota.

## Motivação

O Manus Calendar nasceu de uma necessidade da minha esposa. Os aplicativos de tarefas que ela encontrava exibiam tantas informações que acabavam tirando o foco do que ela precisava fazer. Ela queria um aplicativo minimalista, com apenas o essencial e pouquíssima customização — no máximo, mudar as cores.

Essa necessidade motivou o projeto e orienta sua proposta: ajudar a organizar tarefas e compromissos com uma interface simples, sem excesso de informações ou opções.

## Funcionalidades

- Semana e mês em FullCalendar; ano com 12 mini calendários memoizados, indicadores de registros e acesso direto à semana.
- Hoje, navegação por período, mini calendário independente, barra lateral recolhível e restauração da última visão.
- Criar, consultar, editar, excluir e duplicar tarefas/eventos; prioridade de tarefas e conclusão/reabertura diretamente no calendário.
- Categorias com nome e cor; exclusão de categoria preserva os registros e limpa as referências das exceções.
- Formulários React Hook Form/Zod, validação nativa adicional, mensagens pt-BR e erros de persistência visíveis.
- Recorrências diárias, dias úteis, semanais, mensais, anuais e personalizadas, sem criar cópias permanentes de todo o futuro.
- Alterações em uma ocorrência, naquela e nas próximas ou na série. A conclusão é independente por ocorrência.
- Vários lembretes relativos e lembretes em data/horário absoluto para registros sem repetição ou ocorrências individuais.
- Agendador Rust, notificações pelo plugin oficial, bandeja e autostart opcional, desativado por padrão.
- Temas claro/escuro/sistema, calendário, notificações e comportamento da janela nas configurações.
- Backup JSON local com validação, prévia, confirmação explícita e restauração transacional.

Nenhum registro de demonstração é inserido no banco do aplicativo. O navegador sem Tauri informa a indisponibilidade do armazenamento; não usa localStorage como substituto do SQLite.

## Pré-requisitos

1. Node.js 22.18+ e npm; o ambiente de desenvolvimento utilizado tem Node 24.
2. Rust estável com toolchain MSVC de 64 bits; Tauri/plugins utilizados exigem Rust 1.90 ou superior.
3. Visual Studio Build Tools com **Desenvolvimento para desktop com C++**, MSVC e Windows SDK.
4. Microsoft Edge WebView2 Runtime.
5. Python 3.10+ para os testes independentes das migrations e geração opcional do ícone.

Veja os [pré-requisitos oficiais do Tauri](https://v2.tauri.app/start/prerequisites/). Abra um novo PowerShell depois de instalar o Rust para atualizar o PATH. Use `npm.cmd` se a política do PowerShell bloquear `npm.ps1`; não é necessário mudar a política do sistema.

## Instalar e executar

```powershell
npm.cmd ci
cargo fetch --manifest-path src-tauri/Cargo.toml
npm.cmd run tauri dev
```

A instalação inicial precisa acessar npm/crates.io. A execução do aplicativo instalado é offline. Os plugins Notification, Autostart, Dialog e Single Instance são utilizados pelo lado Rust; não é necessário instalar seus bindings JavaScript.

O `package-lock.json` e o lockfile independente de `calendar-core` estão presentes. O `src-tauri/Cargo.lock` preserva a resolução anterior e precisa ser atualizado pelo Cargo quando as novas dependências nativas puderem ser baixadas. Não foi editado manualmente para simular uma resolução bem-sucedida. Depois dessa instalação, preserve o lockfile completo na distribuição do projeto.

`npm.cmd run dev` serve apenas o frontend para desenvolvimento. O servidor Vite não é incluído no aplicativo empacotado, que carrega arquivos locais. O SQLite é aberto e migrado pelo plugin SQL do Tauri.

## Usar

Clique em **Criar**, em uma célula do mês ou em um horário da semana. A seleção de um intervalo abre o contexto para criar um registro com aquelas datas. Clicar em um registro mostra detalhes e ações; o checkbox de uma tarefa conclui ou reabre a ocorrência.

O formulário mostra datas finais **inclusivas** para registros de dia inteiro; internamente o término é exclusivo. Tarefas sem horário ficam na seção de dia inteiro. Uma tarefa com início e sem término tem duração visual de 30 minutos, sem inventar um horário final persistido. Lembretes de registros sem horário usam 09:00 no fuso do registro.

O botão ao lado de **Minhas categorias** abre o gerenciamento de categorias. Nas configurações, **Salvar configurações** aplica as preferências e a opção de iniciar com o Windows. Mudar a visualização padrão também a seleciona; depois disso, o aplicativo volta a lembrar a última visão utilizada.

## Recorrências e datas

O motor é nativo e compartilhado entre calendário e agendador. Ele implementa o subconjunto do [RFC 5545](https://www.rfc-editor.org/rfc/rfc5545.html) correspondente às opções do MVP:

- `FREQ=DAILY|WEEKLY|MONTHLY|YEARLY`;
- `INTERVAL`, `BYDAY` sem ordinais, `WKST`, `COUNT` ou `UNTIL`;
- dias úteis como segunda a sexta, sem calendário externo de feriados;
- término por data inclusiva, quantidade ou sem término.

Campos RRULE fora desse conjunto são rejeitados explicitamente. Não há importação de calendários ICS neste MVP. Na repetição personalizada mensal/anual, selecionar dias da semana inclui todos os dias selecionados daquele período; sem seleção, usa o dia/mês da âncora. A regra semanal usa `WKST=MO` quando não há outro valor na regra.

Datas inexistentes são puladas, sem consumir `COUNT`: mensal no dia 31 não vira automaticamente dia 28, e 29/02 anual reaparece em anos bissextos. A geração ocorre somente para o intervalo consultado. Identificadores de ocorrências combinam a série e seu início original, independentemente de uma ocorrência ser movida.

Editar **esta e as próximas** cria uma nova âncora e corta a série anterior, preservando a quantidade restante quando a regra não mudou. Exceções futuras e suas conclusões são transferidas. Editar **toda a série** preserva os ajustes individuais; uma regra nova pode deixar datas antigas fora da nova regra. Duplicar um registro recorrente duplica somente a ocorrência escolhida, como registro sem repetição.

As séries guardam o fuso IANA da criação. Inícios recorrentes seguem o horário local desse fuso, e a exibição de registros com horário usa o fuso atual do sistema. Horários inexistentes em transições são pulados; um início ambíguo usa a primeira ocorrência do relógio. A duração exata e os instantes UTC preservam o término correto, inclusive quando o relógio volta. A data final de uma regra com horário é normalizada para `UNTIL` UTC. A edição conserva as referências UTC enquanto os campos de horário não mudam.

## Lembretes, bandeja e inicialização

O agendador vive no Rust e verifica o banco a cada 15 segundos, além de ser acordado pelas alterações dos registros/configurações e pelo foco da janela. Depois da suspensão, a próxima execução reavalia os horários reais. Ele recupera lembretes vencidos nas últimas 24 horas; datas mais antigas são ignoradas para evitar uma avalanche de avisos. Criar/alterar um registro com lembrete já vencido não dispara retroativamente esse lembrete.

A entrega é reservada atomicamente no SQLite antes de chamar o plugin Notification. Entregas confirmadas não são repetidas, nem ao restaurar um backup mais antigo sobre o mesmo histórico. Falhas conhecidas têm até três tentativas. Uma interrupção entre a reserva e a confirmação é conservada como entrega incerta e não é reenviada automaticamente; isso prioriza evitar duplicações, podendo perder um aviso nesse caso raro. O Windows ainda pode suprimir avisos conforme suas configurações.

**O aplicativo totalmente encerrado não emite notificações.** Não há serviço Windows, tarefa agendada ou agendamento independente do processo. Mantenha-o na bandeja ou ative a inicialização com o Windows. O menu da bandeja permite abrir e encerrar completamente. A opção de autostart só altera o Windows quando o usuário a salva; iniciar por ela passa `--autostart` e abre a janela por padrão. Para abrir o calendário ao entrar na conta, ative **Configurações → Sistema → Iniciar Manus Calendar com o Windows** e deixe **Iniciar minimizado na bandeja** desmarcado. A inicialização automática continua desativada por padrão.

Ao reabrir, o aplicativo restaura a data civil selecionada, a visualização, os filtros de tipos e categorias, a barra lateral e o modo minimalista. Aparência e exibição de tarefas concluídas também permanecem salvas. A janela recupera tamanho, posição e estado maximizado; se o monitor anterior não estiver disponível, ela volta para uma posição acessível. A sessão e a geometria ficam na tabela SQLite de configurações, sem novo banco ou migration. Fechar a janela ou encerrar pela bandeja aguarda a gravação da sessão. Desligamentos abruptos podem interromper gravações pendentes.

O teste de notificação está nas configurações. No Windows, o [plugin oficial de notificações](https://v2.tauri.app/plugin/notification/) exige considerar a instalação do aplicativo para o teste real; não trate um teste no navegador como confirmação de entrega nativa.

## Banco e arquitetura

Banco: `sqlite:todo-calendar.db`, resolvido pelo plugin em `app_config_dir`, normalmente `%APPDATA%\br.com.todocalendar.desktop\todo-calendar.db`. O caminho efetivo aparece nas configurações. O banco nunca é criado na pasta da instalação. Consulte o [plugin SQL](https://v2.tauri.app/plugin/sql/).

As seis tabelas principais são `calendar_entries`, `categories`, `recurrence_exceptions`, `reminders`, `notification_deliveries` e `app_settings`. Há quatro migrations versionadas. A quarta preserva registros, relações e histórico ao acrescentar a duração exata; um teste de atualização com dados antigos verifica essa preservação.

O Rust reutiliza o pool SQLite administrado pelo plugin SQL. Mutações relacionadas usam uma transação com conexão dedicada; não há `BEGIN`/`COMMIT` em chamadas JS soltas no pool. As consultas são parametrizadas. Foreign keys são ativadas pelo SQLx e verificadas no bootstrap. Os comandos nativos serializam mutações, consultas compostas e emissão de lembretes para evitar estados intermediários.

```text
src/app/                      Composição React
src/components/calendar/      Semana/mês, ano, contexto e ações
src/components/entries/       Formulário de registros e categorias
src/components/ui/            Componentes shadcn/ui do projeto
src/features/                 Schemas Zod e regras de apresentação
src/services/                 Repositórios e comandos nativos tipados
src/stores/                   Estado de interface e preferências em Zustand
src/hooks/                    Ciclo de vida, filtros, dados e tema
src-tauri/calendar-core/      Modelos, RRULE, persistência, lembretes e backup testáveis
src-tauri/src/                 IPC, fuso IANA, scheduler, bandeja e diálogos nativos
src-tauri/migrations/          Evolução do SQLite
src-tauri/capabilities/        Permissões restritas à janela principal
tests/                        Vitest/Node e migrations SQLite
```

O frontend não recebe acesso genérico a arquivos, shell, rede, notificações ou autostart. Os seletores de arquivo e as integrações do sistema são comandados pelo Rust. O aplicativo não habilita atualização remota. Regras do projeto estão em [AGENTS.md](AGENTS.md).

## Backup

Em **Configurações → Dados**, exporte um JSON ou selecione um para importar. O formato atual é `todo-calendar`, versão de arquivo 1, schema 4. O arquivo contém registros, categorias, exceções, lembretes, histórico de entregas e preferências.

O arquivo é limitado a 20 MB e validado em estrutura, IDs, relações, regras, horários, fuso e configurações antes da confirmação. A prévia recebe um token de uso único; a importação utiliza exatamente os dados revisados. A substituição ocorre em transação e falhas fazem rollback. A configuração atual de autostart não é substituída pelo backup. O histórico atual de entregas confirmadas/incertas é preservado para impedir reenvios.

Limites de entrada: até 12 lembretes por registro, antecedência relativa de até 525.600 minutos, intervalo de repetição de 1 a 999 e `COUNT` de até 100.000. Datas iniciais ficam entre 1900 e 9998. Esses limites são validados em vez de truncar dados silenciosamente.

## Testar

```powershell
npm.cmd run typecheck
npm.cmd test
npm.cmd run test:database
npm.cmd run test:native:core
npm.cmd run test:native
npm.cmd run build
```

O núcleo usa SQLite real para CRUD, transações, recorrências, lembretes, backup, falhas de escrita/restauração e reabertura de arquivo. Testes de transição de horário usam adaptadores controlados. Os testes IANA reais ficam em `src-tauri/src/timezone.rs` e precisam do build desktop com `chrono-tz` disponível.

Para o ambiente restrito desta implementação:

```powershell
npm.cmd run test:restricted
npm.cmd run build:validation
```

Esses scripts executam o **Vitest real** e o **Vite real**, substituindo somente a transformação por subprocesso do esbuild pelo compilador TypeScript em processo. O bundle de verificação é não minificado, usa `.validation-dist` e não é um instalador. O teste Node `npm.cmd run test:foundation` é uma rota adicional e compartilha casos com o Vitest; não some os dois resultados como testes independentes.

`npm.cmd run preview:validation` serve os assets de verificação para desenvolvimento. `.qa/editor.html` é uma prévia isolada do formulário, sem persistência e fora do build de produção. Nenhum driver de teste é incluído na persistência do aplicativo.

## Gerar o instalador

Em um Windows com as dependências baixadas e o build padrão funcionando:

```powershell
cargo fetch --manifest-path src-tauri/Cargo.toml
npm.cmd run desktop:build
```

O Tauri executa o typecheck/build do frontend e compila o aplicativo nativo. A configuração solicita instalador **NSIS** em português brasileiro, nome **Manus Calendar**, identificador `br.com.todocalendar.desktop` e ícone original. O instalador deve aparecer em `src-tauri/target/release/bundle/nsis/`. Essa saída ainda não foi produzida neste ambiente. Veja a [documentação de instaladores Windows](https://v2.tauri.app/distribute/windows-installer/).

Os ícones PNG/ICO são reproduzíveis com `python scripts/generate-icon.py`. Não há assinatura de produção nem atualização automática remota configurada.

### Aparência do calendário

- No seletor de visualização, **Semana com horários** mantém a agenda por hora; **Semana em grade** mostra apenas os sete dias, no estilo da visão mensal, sem a escala de horários. As duas opções preservam a data e os filtros, lembram a última escolha e podem ser definidas como padrão nas configurações.
- Use o botão **Modo minimalista** no canto superior direito para ocultar o cabeçalho, a lateral e os textos auxiliares. O mesmo botão restaura a interface, mantendo a data, a visualização e os filtros. Esse modo vale para a sessão atual.
- Em **Configurações → Aparência → Cores do calendário**, escolha as cores do texto e do fundo, confira a prévia e clique em **Salvar configurações**. As cores são salvas no SQLite e aplicadas às visões de mês, semana e ano. As categorias mantêm suas próprias cores.
- **Restaurar cores do tema** volta às cores do tema claro, escuro ou do sistema após salvar.
