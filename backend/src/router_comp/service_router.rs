use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json};
use rand::distributions::Alphanumeric;
use rand::prelude::ThreadRng;
use rand::Rng;
use serde::{Deserialize};
use uuid::Uuid;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateService {
    name: String,
}


pub async fn create_service(
    State(state): State<AppState>,
    Json(create_service): Json<CreateService>,
) -> impl IntoResponse {

    let api_key = generate_api_key(16);

    sqlx::query(
        "INSERT INTO services (id, name, api_key) VALUES ($1, $2, $3) RETURNING *"
    )
        .bind(Uuid::new_v4())
        .bind(create_service.name)
        .bind(&api_key)
        .fetch_one(&state.postgres)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR).unwrap();

    (StatusCode::CREATED, format!("Service created with API key: {}", api_key)).into_response()
}
pub fn generate_api_key(length: usize) -> String {
    let mut rng = rand::thread_rng();
    generate_api_key_with_rng(length, &mut rng)
}

fn generate_api_key_with_rng(length: usize, rng: &mut ThreadRng) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub async fn delete_service(
    Path(service_id): Path<Uuid>,
    State(state): State<AppState>) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM services WHERE id = $1")
        .bind(service_id)
        .execute(&state.postgres)
        .await;
    match result {
        Ok(_) => {
            if result.unwrap().rows_affected() > 0 {
                (StatusCode::OK, "Service deleted".to_string()).into_response()
            } else {
                (StatusCode::NOT_FOUND, "Service not found".to_string()).into_response()
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete service: {}", e),
        )
            .into_response(),
    }
}