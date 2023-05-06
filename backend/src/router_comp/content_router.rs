use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse},
    Json};

use serde::{Serialize, Deserialize};
use sqlx::{Row};
use uuid::Uuid;
use anyhow::Result;
use crate::AppState;


#[derive(Deserialize)]
pub struct NewContentType {
    name: String,
}

#[derive(Deserialize)]
pub struct ContentType {
    content_type_id: Uuid,
}

#[derive(Deserialize)]
pub struct NewField {
    content_type_id: Uuid,
    display_name: String,
    field_type: String,
    required: bool,
}

#[derive(Deserialize)]
pub struct NewContentItem {
    content_type_id: Uuid,
    data: serde_json::Value,
}

#[derive(Serialize)]
struct ContentItem {
    id: Uuid,
    data: serde_json::Value,
}

pub async fn create_content_type(
    Path(service_id): Path<String>,
    State(state): State<AppState>,
    Json(new_content_type): Json<NewContentType>,
) -> impl IntoResponse {
    let query = sqlx::query(
        "INSERT INTO content_types (id, name, service_id)
         VALUES ($1, $2, $3)",
    )
        .bind(Uuid::new_v4())
        .bind(new_content_type.name)
        .bind(service_id)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::CREATED, "コンテンツタイプが作成されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツタイプの作成に失敗しました: {}", e),
        ).into_response()
    }
}

pub async fn create_field(
    State(state): State<AppState>,
    Json(new_field): Json<NewField>,
) -> impl IntoResponse {
    let query = sqlx::query("INSERT INTO fields (id, content_type_id,display_id, field_type, required)
                                    VALUES ($1, $2, $3, $4, $5)",
    )
        .bind(Uuid::new_v4())
        .bind(new_field.content_type_id)
        .bind(new_field.display_name)
        .bind(new_field.field_type)
        .bind(new_field.required)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::CREATED, "フィールドが作成されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("フィールドの作成に失敗しました: {}", e),
        )
            .into_response(),
    }
}

pub async fn create_content_item(
    State(state): State<AppState>,
    Json(new_content_item): Json<NewContentItem>,
) -> impl IntoResponse {
    let query = sqlx::query(
        "INSERT INTO content_items (id, content_type_id, data)
        VALUES ($1, $2, $3)",
    )
        .bind(Uuid::new_v4())
        .bind(new_content_item.content_type_id)
        .bind(new_content_item.data)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::CREATED, "コンテンツアイテムが作成されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの作成に失敗しました: {}", e),
        )
            .into_response(),
    }
}



pub async fn get_content_items(
    Path(_service_id): Path<String>,
    State(state): State<AppState>,
    Json(content_type): Json<ContentType>,
) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT id, data FROM content_items WHERE content_type_id = $1"
    )
        .bind(content_type.content_type_id)
        .fetch_all(&state.postgres)
        .await;

    match rows {
        Ok(items) => {
            if items.is_empty() {
                return (StatusCode::NOT_FOUND, "コンテンツアイテムが見つかりませんでした".to_string()).into_response();
            }
            let content_items: Result<Vec<ContentItem>> = items
                .into_iter()
                .map(|row| {
                    Ok(ContentItem {
                        id: row.try_get("id")?,
                        data: row.try_get("data")?,
                    })
                })
                .collect();

            match content_items {
                Ok(items) => Json(items).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error converting content items: {}", e),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの取得に失敗しました: {}", e),
        )
            .into_response(),
    }
}