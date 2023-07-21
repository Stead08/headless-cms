mod libs;
mod models;
mod router;
mod router_comp;

use std::env;
use crate::router::create_router;
use anyhow::Error;
use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use hyper_tls::HttpsConnector;
use jsonwebtoken::jwk::JwkSet;
use sea_orm::{DatabaseConnection, SqlxPostgresConnector};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use std::time::Duration;
use hyper::body::to_bytes;
use hyper::Client;

use tracing_subscriber::fmt;

#[derive(Clone)]
pub struct AppState {
    postgres: DatabaseConnection,
    pgpool: PgPool,
    key: Key,
    smtp_email: String,
    smtp_password: String,
    domain: String,
    authority: String,
    client_id: String,
    audience: String,
    issuer: String,
    jwks: JwkSet,
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    fmt::init();
    let db_address = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let postgres = PgPoolOptions::new()
        .max_connections(5)
        .idle_timeout(Some(Duration::from_secs(1)))
        .connect(&db_address)
        .await
        .expect("Failed to connect to Postgres!");

    let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(postgres.clone());

    sqlx::migrate!()
        .run(&postgres)
        .await
        .expect("Failed to run migrations!");

    let smtp_email = std::env::var("SMTP_EMAIL").expect("You need to set your SMTP_EMAIL secret!");

    let smtp_password =
        std::env::var("SMTP_PASSWORD").expect("You need to set your SMTP_PASSWORD secret!");

    let domain = std::env::var("DOMAIN").expect("You need to set your DOMAIN secret!");

    let authority = std::env::var("AUTHORITY").expect("AUTHORITY must be set");

    let audience = std::env::var("AUDIENCE").expect("AUDIENCE must be set");

    let issuer = std::env::var("ISSUER").expect("ISSUER must be set");
    let client_id = std::env::var("AUTH0_CLIENT_ID").expect("CLIENT_ID must be set");
    let client_secret = std::env::var("AUTH0_CLIENT_SECRET").expect("CLIENT_SECRET must be set");
    let jwks = get_jwks(&authority).await.expect("failed to fetch jwks");

    let state = AppState {
        postgres: conn,
        pgpool: postgres,
        key: Key::generate(),
        smtp_email,
        smtp_password,
        domain,
        authority,
        client_id,
        audience,
        issuer,
        jwks,
    };

    let router = create_router(state);

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a number!");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Listening on Port: {}", port);
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn get_jwks(authority: &str) -> anyhow::Result<JwkSet, Error> {
    //fetch jwks
    let jwks_uri = format!("{}{}", authority, "/.well-known/jwks.json")
        .parse()
        .expect("Invalid uri");

    let https = HttpsConnector::new();
    let client = Client::builder().build::<_, hyper::Body>(https);
    let response = client.get(jwks_uri).await.expect("failed to fetch jwks");

    let body_bytes = to_bytes(response.into_body()).await?;
    let jwks: JwkSet = serde_json::from_slice(&body_bytes)?;
    Ok(jwks)
}
