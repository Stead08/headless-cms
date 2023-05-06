use std::collections::HashSet;
use std::{fmt, result};
use std::fmt::{Display, Formatter};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json};
use rand::distributions::Alphanumeric;
use rand::prelude::ThreadRng;
use rand::Rng;
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::AppState;

#[derive(Deserialize)]
pub struct CreateService {
    name: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Permission {
    Post,
    Get,
    Put,
    Patch,
    Delete,
}

impl Display for Permission {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Permission::Post => write!(f, "Post"),
            Permission::Get => write!(f, "Get"),
            Permission::Put => write!(f, "Put"),
            Permission::Patch => write!(f, "Patch"),
            Permission::Delete => write!(f, "Delete"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

#[derive(sqlx::FromRow)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub api_key: String,
}

pub async fn create_service(
    State(state): State<AppState>,
    Json(create_service): Json<CreateService>,
) -> impl IntoResponse {
    let service_id = generate_key(16);
    let api_key = generate_key(32);

    let result = sqlx::query_as::<_, Service>(
        "INSERT INTO services (id, name, api_key) VALUES ($1, $2, $3) RETURNING *"
    )
        .bind(&service_id)
        .bind(create_service.name)
        .bind(&api_key)
        .fetch_one(&state.postgres)
        .await;

    match result {
        Ok(service) => (StatusCode::CREATED, format!("Service {} created with \n API key: {} \n Service ID: {}", service.name, service.api_key, service.id)).into_response(),
        Err(e) => {
            println!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub fn generate_key(length: usize) -> String {
    let mut rng = rand::thread_rng();
    generate_api_key_with_rng(length, &mut rng)
}

fn generate_api_key_with_rng(length: usize, rng: &mut ThreadRng) -> String {
    rng.sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

pub async fn create_role(
    Path(service_id): Path<String>,
    State(state): State<AppState>,
    role: Json<Role>,
) -> impl IntoResponse {
    let role_id = Uuid::new_v4();

    let role_result = sqlx::query(
        "INSERT INTO roles (id, name, service_id) VALUES ($1, $2, $3)"
    )
        .bind(role_id)
        .bind(&role.name)
        .bind(service_id)
        .execute(&state.postgres)
        .await;

    if role_result.is_err() {
        eprint!("{}", role_result.err().unwrap());
        return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
    }

    for permission in role.permissions.iter() {
        let permission_result = sqlx::query(
            "INSERT INTO role_permissions (role_id, permission) VALUES ($1, $2)"
        )
            .bind(role_id)
            .bind(permission.to_string())
            .execute(&state.postgres)
            .await;

        if permission_result.is_err() {
            eprint!("{}", permission_result.err().unwrap());
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }

    (StatusCode::CREATED).into_response()
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