mod libs;
mod models;
mod router;
mod router_comp;

use std::net::SocketAddr;
use std::time::Duration;
use crate::router::create_router;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use sea_orm::{
    DatabaseConnection,
    SqlxPostgresConnector,
};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

#[derive(Clone)]
pub struct AppState {
    postgres: DatabaseConnection,
    pgpool: PgPool,
    key: Key,
    smtp_email: String,
    smtp_password: String,
    domain: String,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[tokio::main]
async fn main(
) -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let db_address = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let postgres = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(Some(Duration::from_secs(1)))
        .connect(&db_address)
        .await.expect("Failed to connect to Postgres!");

    let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres.clone());

    sqlx::migrate!()
        .run(&postgres)
        .await
        .expect("Failed to run migrations!");


    let smtp_email = std::env::var("SMTP_EMAIL")
        .expect("You need to set your SMTP_EMAIL secret!");

    let smtp_password = std::env::var("SMTP_PASSWORD")
        .expect("You need to set your SMTP_PASSWORD secret!");

    let domain = std::env::var("DOMAIN")
        .expect("You need to set your DOMAIN secret!");

    let state = AppState {
        postgres: conn,
        pgpool: postgres,
        key: Key::generate(),
        smtp_email,
        smtp_password,
        domain,
    };

    let router = create_router(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number!");
    let addr = SocketAddr::from(([127, 0, 0, 1], port));

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();

    Ok(())
}

