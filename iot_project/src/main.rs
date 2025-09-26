use axum::{
    routing::{get, post},
    extract::State,
    response::{Html, Json},
    Json as AxumJson, Router,
};
use sqlx::PgPool;
use uuid::Uuid;
use dotenvy::dotenv;
use std::env;
use tokio::net::TcpListener;

mod db;
mod models;

use db::{Item, init_db};
use models::CreateItem;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let db_url = env::var("DATABASE_URL")
        .expect("❌ DATABASE_URL não definido no .env");

    // Conecta ao Postgres
    let pool = PgPool::connect(&db_url).await?;
    init_db(&pool).await?;
    println!("✅ Conectado ao banco PostgreSQL em: {}", db_url);

    // Rotas
    let app = Router::new()
        .route("/", get(list_page))
        .route("/items", post(create_item).get(list_items))
        .with_state(pool);

    // Porta 0 = random
    let listener = TcpListener::bind("0.0.0.0:0").await?;
    let addr = listener.local_addr()?;
    println!("🚀 Servidor rodando em http://{}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

// POST /items
async fn create_item(
    State(pool): State<PgPool>,
    AxumJson(payload): AxumJson<CreateItem>
) -> Json<Item> {
    let item = Item {
        id: Uuid::new_v4().to_string(),
        nome: payload.nome,
    };

    sqlx::query("INSERT INTO items (id, nome) VALUES ($1, $2)")
        .bind(&item.id)
        .bind(&item.nome)
        .execute(&pool)
        .await
        .unwrap();

    Json(item)
}

// GET /items
async fn list_items(State(pool): State<PgPool>) -> Json<Vec<Item>> {
    let items = sqlx::query_as::<_, Item>("SELECT id::text, nome FROM items")
        .fetch_all(&pool)
        .await
        .unwrap();
    Json(items)
}

// GET /
async fn list_page(State(pool): State<PgPool>) -> Html<String> {
    let items = sqlx::query_as::<_, Item>("SELECT id::text, nome FROM items")
        .fetch_all(&pool)
        .await
        .unwrap();

    let mut html = String::from("<h1>Lista de Itens</h1><ul>");
    for item in items {
        html.push_str(&format!("<li>{}: {}</li>", item.id, item.nome));
    }
    html.push_str("</ul>");
    Html(html)
}
