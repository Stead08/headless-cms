use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, EntityTrait};
use std::collections::HashSet;
use std::fmt;
use std::fmt::{Display, Formatter};

use crate::libs::generate_random_key::generate_key;
use crate::{models, AppState};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateService {
    name: String,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

#[derive(Serialize, Deserialize)]
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

    let new_service = models::services::ActiveModel {
        id: Set(service_id.clone()),
        name: Set(create_service.name),
        api_key: Set(api_key.clone()),
    };

    let service_result = new_service.insert(&state.postgres).await;

    match service_result {
        Ok(service) => {
            let new_role = models::roles::ActiveModel {
                id: Default::default(),
                name: Set("Admin".to_string()),
                service_id: Set(service_id),
                api_key: Set(api_key),
            };

            let role_result = new_role.insert(&state.postgres).await;

            let role_id: i32 = match role_result {
                Ok(role) => role.id,
                Err(e) => {
                    eprint!("{}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            };

            let permissions = [
                Permission::Post,
                Permission::Get,
                Permission::Put,
                Permission::Patch,
                Permission::Delete,
            ];

            for permission in permissions.iter() {
                let new_permission = models::role_permissions::ActiveModel {
                    role_id: Set(role_id),
                    permission: Set(permission.to_string()),
                };

                let permission_result = new_permission.insert(&state.postgres).await;

                if permission_result.is_err() {
                    eprint!("{}", permission_result.err().unwrap());
                    return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            }

            (
                StatusCode::CREATED,
                format!(
                    "Service {} created with \n API key: {} \n Service ID: {}",
                    service.name, service.api_key, service.id
                ),
            )
                .into_response()
        }
        Err(e) => {
            println!("{}", e);
            (StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}

pub async fn create_role(
    Path(service_id): Path<String>,
    State(state): State<AppState>,
    role: Json<Role>,
) -> impl IntoResponse {
    let api_key = generate_key(32);

    let new_role = models::roles::ActiveModel {
        id: Default::default(),
        name: Set(role.name.clone()),
        service_id: Set(service_id),
        api_key: Set(api_key.clone()),
    };

    let role_result = new_role.insert(&state.postgres).await;

    let role_id: i32 = match role_result {
        Ok(role) => role.id,
        Err(e) => {
            eprint!("{}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    for permission in role.permissions.iter() {
        let new_permission = models::role_permissions::ActiveModel {
            role_id: Set(role_id),
            permission: Set(permission.to_string()),
        };

        let permission_result = new_permission.insert(&state.postgres).await;

        if permission_result.is_err() {
            eprint!("{}", permission_result.err().unwrap());
            return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }

    (StatusCode::CREATED, api_key).into_response()
}

pub async fn delete_service(
    Path(service_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let result = models::prelude::Services::delete_by_id(service_id)
        .exec(&state.postgres)
        .await;

    match result {
        Ok(_) => {
            if result.unwrap().rows_affected > 0 {
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
