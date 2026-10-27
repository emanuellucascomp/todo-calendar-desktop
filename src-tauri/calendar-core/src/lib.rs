pub mod backup;
pub mod models;
pub mod recurrence;
pub mod reminders;
pub mod repository;
pub mod settings;
pub const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/0001_initial.sql"),
    include_str!("../../migrations/0002_occurrences.sql"),
    include_str!("../../migrations/0003_delivery_attempts.sql"),
    include_str!("../../migrations/0004_exact_duration.sql"),
];

pub type Result<T> = std::result::Result<T, String>;
pub fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
