use sqlx::{PgPool, FromRow};

#[derive(FromRow, Debug, serde::Serialize)]
pub struct Item {
    pub id: String,
    pub nome: String,
}

pub async fn init_db(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS items (
            id UUID PRIMARY KEY,
            nome TEXT NOT NULL
        )
        "#
    )
        .execute(pool)
        .await?;
    Ok(())
}
