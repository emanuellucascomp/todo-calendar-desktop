use crate::{error, now, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub first_day: i64,
    pub default_view: String,
    pub show_completed: bool,
    pub notifications_enabled: bool,
    pub default_reminder_minutes: i64,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub close_behavior: String,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            first_day: 0,
            default_view: "month".into(),
            show_completed: true,
            notifications_enabled: true,
            default_reminder_minutes: 10,
            autostart: false,
            minimize_to_tray: false,
            close_behavior: "exit".into(),
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<()> {
        if !["light", "dark", "system"].contains(&self.theme.as_str())
            || ![0, 1].contains(&self.first_day)
            || !["month", "week", "year"].contains(&self.default_view.as_str())
            || !["exit", "tray"].contains(&self.close_behavior.as_str())
            || !(0..=525_600).contains(&self.default_reminder_minutes)
        {
            return Err("Configurações inválidas.".into());
        }
        Ok(())
    }
}
pub async fn load(pool: &SqlitePool) -> Result<Preferences> {
    let rows = sqlx::query_as::<_, (String, String)>("SELECT key,value FROM app_settings")
        .fetch_all(pool)
        .await
        .map_err(error)?;
    let mut value = serde_json::to_value(Preferences::default()).map_err(error)?;
    for (key, json) in rows {
        if value.get(&key).is_some() {
            value[&key] = serde_json::from_str(&json).map_err(error)?;
        }
    }
    let preferences: Preferences = serde_json::from_value(value).map_err(error)?;
    preferences.validate()?;
    Ok(preferences)
}
pub async fn save(pool: &SqlitePool, preferences: &Preferences) -> Result<()> {
    preferences.validate()?;
    let previous = load(pool).await?;
    let mut tx = pool.begin().await.map_err(error)?;
    for (key, value) in serde_json::to_value(preferences)
        .map_err(error)?
        .as_object()
        .unwrap()
    {
        sqlx::query("INSERT INTO app_settings(key,value) VALUES(?,?) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=?").bind(key).bind(value.to_string()).bind(now()).execute(&mut *tx).await.map_err(error)?;
    }
    if previous.default_view != preferences.default_view {
        sqlx::query("UPDATE app_settings SET value=? WHERE key='last_view'")
            .bind(serde_json::to_string(&preferences.default_view).map_err(error)?)
            .execute(&mut *tx)
            .await
            .map_err(error)?;
    }
    if !preferences.notifications_enabled {
        sqlx::query("UPDATE notification_deliveries SET state='cancelled' WHERE state IN ('pending','failed')").execute(&mut *tx).await.map_err(error)?;
    }
    tx.commit().await.map_err(error)
}
