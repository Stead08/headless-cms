mod libs;
mod models;
mod router;
mod router_comp;

use crate::router::create_router;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use sea_orm::{
    DatabaseConnection,
    SqlxPostgresConnector
};
use shuttle_secrets::SecretStore;
use sqlx::PgPool;

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

#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres] postgres: PgPool,
    #[shuttle_secrets::Secrets] secrets: SecretStore,
) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!()
        .run(&postgres)
        .await
        .expect("Failed to run migrations!");
    let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres.clone());

    let smtp_email = secrets
        .get("SMTP_EMAIL")
        .expect("You need to set your SMTP_EMAIL secret!");

    let smtp_password = secrets
        .get("SMTP_PASSWORD")
        .expect("You need to set your SMTP_PASSWORD secret!");

    let domain = secrets
        .get("DOMAIN")
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

    Ok(router.into())
}

