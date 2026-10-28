mod backup;
mod calendar;
mod commands;
mod runtime;
mod timezone;
mod tray;
mod window_session;

use tauri_plugin_sql::{Migration, MigrationKind};
use tauri::Manager;

const DATABASE_URL: &str = "sqlite:todo-calendar.db";

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let start_in_tray = app.state::<runtime::RuntimeState>().start_in_tray.load(std::sync::atomic::Ordering::Relaxed);
            if window_session::should_show(args.iter().map(String::as_str), start_in_tray) {
                tray::show(app);
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name("Manus Calendar")
                .arg("--autostart")
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(
                    DATABASE_URL,
                    vec![
                        Migration {
                            version: 1,
                            description: "initial_schema",
                            sql: include_str!("../migrations/0001_initial.sql"),
                            kind: MigrationKind::Up,
                        },
                        Migration {
                            version: 2,
                            description: "occurrence_metadata",
                            sql: include_str!("../migrations/0002_occurrences.sql"),
                            kind: MigrationKind::Up,
                        },
                        Migration {
                            version: 3,
                            description: "delivery_attempts",
                            sql: include_str!("../migrations/0003_delivery_attempts.sql"),
                            kind: MigrationKind::Up,
                        },
                        Migration {
                            version: 4,
                            description: "exact_duration",
                            sql: include_str!("../migrations/0004_exact_duration.sql"),
                            kind: MigrationKind::Up,
                        },
                        Migration {
                            version: 5,
                            description: "entry_typography",
                            sql: include_str!("../migrations/0005_entry_typography.sql"),
                            kind: MigrationKind::Up,
                        },
                    ],
                )
                .build(),
        )
        .manage(calendar::MutationGate::default())
        .manage(runtime::RuntimeState::default())
        .manage(window_session::WindowSession::default())
        .setup(|app| {
            tray::install(app.handle()).map_err(std::io::Error::other)?;
            let preferences = tauri::async_runtime::block_on(async {
                let repo = calendar::repository(app.handle()).await?;
                todo_calendar_core::settings::load(&repo.pool).await
            })
            .map_err(std::io::Error::other)?;
            runtime::apply(app.handle(), &preferences);
            runtime::start(app.handle().clone());
            tauri::async_runtime::block_on(window_session::restore(app.handle()))
                .map_err(std::io::Error::other)?;
            let args: Vec<String> = std::env::args().collect();
            if window_session::should_show(args.iter().map(String::as_str), preferences.start_in_tray) {
                tray::show(app.handle());
            }
            Ok(())
        })
        .on_window_event(tray::window_event)
        .invoke_handler(tauri::generate_handler![
            commands::database_info,
            window_session::session_ready,
            window_session::finish_session_close,
            calendar::list_occurrences,
            calendar::entry_details,
            calendar::save_entry,
            calendar::delete_entry,
            calendar::complete_entry,
            calendar::duplicate_entry,
            calendar::save_category,
            calendar::delete_category,
            runtime::get_settings,
            runtime::save_settings,
            runtime::test_notification,
            runtime::runtime_status,
            tray::hide_to_tray,
            backup::export_backup,
            backup::preview_backup,
            backup::import_backup
        ])
        .run(tauri::generate_context!())
        .expect("Não foi possível iniciar o Manus Calendar");
}
