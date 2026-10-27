use super::*;

impl Repository {
    pub async fn save_category(
        &self,
        identifier: Option<&str>,
        name: &str,
        color: &str,
    ) -> Result<String> {
        if name.trim().is_empty() || name.chars().count() > 80 {
            return Err("Informe um nome de até 80 caracteres.".into());
        }
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Cor inválida.".into());
        }
        let identifier = identifier.map(str::to_owned).unwrap_or_else(id);
        sqlx::query("INSERT INTO categories(id,name,color) VALUES(?,?,?) ON CONFLICT(id) DO UPDATE SET name=excluded.name,color=excluded.color,updated_at=?").bind(&identifier).bind(name.trim()).bind(color).bind(now()).execute(&self.pool).await.map_err(error)?;
        Ok(identifier)
    }
    pub async fn delete_category(&self, identifier: &str) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(error)?;
        sqlx::query("UPDATE recurrence_exceptions SET overrides_json=json_set(overrides_json,'$.category_id',NULL),updated_at=? WHERE json_extract(overrides_json,'$.category_id')=?").bind(now()).bind(identifier).execute(&mut *tx).await.map_err(error)?;
        sqlx::query("UPDATE calendar_entries SET updated_at=? WHERE category_id=?")
            .bind(now())
            .bind(identifier)
            .execute(&mut *tx)
            .await
            .map_err(error)?;
        sqlx::query("DELETE FROM categories WHERE id=?")
            .bind(identifier)
            .execute(&mut *tx)
            .await
            .map_err(error)?;
        tx.commit().await.map_err(error)
    }
}
