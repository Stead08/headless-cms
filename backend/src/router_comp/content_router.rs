use std::collections::HashMap;
use std::{fmt, io};
use std::fmt::{Display, Formatter};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse},
    Json};
use serde::{Serialize, Deserialize};
use sqlx::{Decode, Row, Type};
use uuid::Uuid;
use anyhow::Result;
use serde_json::{json};
use sqlx::postgres::PgTypeInfo;
use crate::AppState;

#[derive(Deserialize)]
pub struct NewContentType {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct ContentType {
    pub id: i32,
    pub name: String,
    pub fields: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Field {
    pub id: i32,
    pub content_type_id: i32,
    pub display_id: String,
    pub field_type: FieldType,
    pub required: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Field {
    pub fn field_type_matches(&self, value: &serde_json::Value) -> bool {
        match self.field_type {
            FieldType::Text => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Date => value.is_string() && {
                let date_string = value.as_str().unwrap();
                chrono::DateTime::parse_from_rfc3339(date_string).is_ok()
            },
            FieldType::Boolean => value.is_boolean(),
            // 他のフィールドタイプについても同様に追加
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
pub enum FieldType {
    Text,
    Number,
    Date,
    Boolean,
    // 他のフィールドタイプも追加予定
}

impl Display for FieldType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            FieldType::Text => write!(f, "Text"),
            FieldType::Number => write!(f, "Number"),
            FieldType::Date => write!(f, "Date"),
            FieldType::Boolean => write!(f, "Boolean"),
        }
    }
}

impl Type<sqlx::Postgres> for FieldType {
    fn type_info() -> PgTypeInfo {
        <String as Type<sqlx::Postgres>>::type_info()
    }

    fn compatible(_ty: &PgTypeInfo) -> bool {
        <String as Type<sqlx::Postgres>>::compatible(_ty)
    }
}

impl<'r> Decode<'r, sqlx::Postgres> for FieldType {
    fn decode(value: <sqlx::Postgres as sqlx::database::HasValueRef<'r>>::ValueRef) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let string_value = <String as Decode<sqlx::Postgres>>::decode(value)?;

        match string_value.as_str() {
            "Text" => Ok(FieldType::Text),
            "Number" => Ok(FieldType::Number),
            "Date" => Ok(FieldType::Date),
            "Boolean" => Ok(FieldType::Boolean),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid field type").into()),
        }
    }
}

#[derive(Deserialize)]
pub struct NewField {
    display_name: String,
    field_type: FieldType,
    required: bool,
}

#[derive(Deserialize)]
pub struct NewContentItem {
    data: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
pub struct ContentItem {
    pub id: Option<Uuid>,
    pub data: HashMap<String, serde_json::Value>,
}

pub async fn create_content_type(
    Path(service_id): Path<String>,
    State(state): State<AppState>,
    Json(new_content_type): Json<NewContentType>,
) -> impl IntoResponse {
    let query = sqlx::query(
        "INSERT INTO content_types (name, service_id)
         VALUES ($1, $2) RETURNING id",
    )
        .bind(new_content_type.name)
        .bind(service_id)
        .fetch_one(&state.postgres);

    match query.await {
        Ok(res) => {
            //問い合わせの返り値からidを取得
            let content_type_id: i32 = res.get("id");
            (StatusCode::CREATED, format!("コンテンツタイプが作成されました \n content_type_id: {}", content_type_id)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツタイプの作成に失敗しました: {}", e),
        ).into_response()
    }
}

pub async fn create_field(
    State(state): State<AppState>,
    Path((_service_id, content_type_id)): Path<(String, i64)>,
    Json(new_field): Json<NewField>,
) -> impl IntoResponse {
    let query = sqlx::query("INSERT INTO fields (content_type_id,display_id, field_type, required)
                                    VALUES ($1, $2, $3, $4)",
    )
        .bind(content_type_id)
        .bind(new_field.display_name)
        .bind(new_field.field_type.to_string())
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

pub async fn get_content_type(
    Path((_service_id, content_type_id)): Path<(String, i32)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let content_type_row = sqlx::query(
        "SELECT id, name FROM content_types WHERE id = $1",
    )
        .bind(content_type_id)
        .fetch_optional(&state.postgres)
        .await;

    match content_type_row {
        Ok(Some(row)) => {
            let fields_rows = sqlx::query_as::<_, Field>(
                "SELECT * FROM fields WHERE content_type_id = $1",
            )
                .bind(content_type_id)
                .fetch_all(&state.postgres)
                .await;

            match fields_rows {
                Ok(fields) => {
                    let fields_json = fields
                        .into_iter()
                        .map(|field| {
                            json!({
                    "display_id": field.display_id,
                    "field_type": field.field_type,
                    "required": field.required
                })
                        })
                        .collect::<Vec<_>>();

                    let content_type = ContentType {
                        id: row.get("id"),
                        name: row.get("name"),
                        fields: fields_json,
                    };
                    Json(content_type).into_response()
                }
                Err(e) => {
                    eprint!("{}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("フィールドの取得に失敗しました: {}", e),
                    )
                }
                    .into_response(),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            "コンテンツタイプが見つかりませんでした".to_string(),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツタイプの取得に失敗しました: {}", e),
        )
            .into_response(),
    }
}


pub async fn create_content_item(
    State(state): State<AppState>,
    Path((_service_id, content_type_id)): Path<(String, i32)>,
    Json(new_content_item): Json<NewContentItem>,
) -> impl IntoResponse {
    let fields = sqlx::query_as::<_, Field>(
        "SELECT * FROM fields WHERE content_type_id = $1",
    )
        .bind(content_type_id)
        .fetch_all(&state.postgres)
        .await
        .unwrap_or_else(|_| Vec::new());

    //データ型の検証
    let mut valid_data = HashMap::new();
    for field in fields {
        if let Some(value) = new_content_item.data.get(&field.display_id) {
            // Check if the value's type matches the field_type
            if field.field_type_matches(value) {
                valid_data.insert(field.display_id.clone(), value.clone());
            } else {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("データ型が一致しません: {}", field.display_id),
                )
                    .into_response();
            }
        } else if field.required {
            return (
                StatusCode::BAD_REQUEST,
                format!("必須フィールドがありません: {}", field.display_id),
            )
                .into_response();
        }
    }


    let query = sqlx::query(
        "INSERT INTO content_items (id, content_type_id, data)
        VALUES ($1, $2, $3)",
    )
        .bind(Uuid::new_v4())
        .bind(content_type_id)
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

pub async fn delete_content_item(
    State(state): State<AppState>,
    Path((_service_id, content_item_id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    let query = sqlx::query("DELETE FROM content_items WHERE id = $1")
        .bind(content_item_id)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::OK, "コンテンツアイテムが削除されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの削除に失敗しました: {}", e),
        )
            .into_response(),
    }
}

pub async fn update_content_item(
    State(state): State<AppState>,
    Path((_service_id, content_item_id)): Path<(String, Uuid)>,
    Json(content_item): Json<ContentItem>,
) -> impl IntoResponse {
    //content_itemが空の場合はBAD_REQUESTを返す
    if content_item.data.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            "コンテンツアイテムが空です".to_string(),
        )
            .into_response();
    }

    //content_item_idでcontent_itemテーブルからcontent_type_idを取得する
    let query = sqlx::query(
        "SELECT content_type_id FROM content_items WHERE id = $1",
    )
        .bind(content_item_id)
        .fetch_one(&state.postgres)
        .await;

    let content_type_id: i32 = match query {
        Ok(row) => row.get("content_type_id"),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "コンテンツが存在しません".to_string(),
            )
                .into_response();
        }
    };

    let query = sqlx::query_as::<_, Field>(
        "SELECT * FROM fields WHERE content_type_id = $1",
    )
        .bind(content_type_id)
        .fetch_all(&state.postgres)
        .await;

    let fields = match query {
        Ok(fields) => fields,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("フィールドの取得に失敗しました: {}", e),
            )
                .into_response();
        }
    };

    //データ型の検証
    let mut valid_data = HashMap::new();
    for field in fields {
        if let Some(value) = content_item.data.get(&field.display_id) {
            // Check if the value's type matches the field_type
            if field.field_type_matches(value) {
                valid_data.insert(field.display_id.clone(), value.clone());
            } else {
                return (
                    StatusCode::BAD_REQUEST,
                    format!("データ型が一致しません: {}", field.display_id),
                )
                    .into_response();
            }
        } else if field.required {
            return (
                StatusCode::BAD_REQUEST,
                format!("必須フィールドがありません: {}", field.display_id),
            )
                .into_response();
        }
    }

    let json_data = json!(valid_data);

    let query = sqlx::query(
        "UPDATE content_items SET data = $1 WHERE id = $2",
    )
        .bind(json_data)
        .bind(content_item_id)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::OK, "コンテンツアイテムが更新されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの更新に失敗しました: {}", e),
        )
            .into_response(),
    }
}

pub async fn get_content_items(
    State(state): State<AppState>,
    Path((_service_id, content_type_id)): Path<(String, i32)>,
) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT id, data FROM content_items WHERE content_type_id = $1"
    )
        .bind(content_type_id)
        .fetch_all(&state.postgres)
        .await;

    match rows {
        Ok(items) => {
            if items.is_empty() {
                return (StatusCode::NOT_FOUND, "コンテンツアイテムが見つかりませんでした".to_string()).into_response();
            }

            let content_items: Result<Vec<ContentItem>, Box<dyn std::error::Error>> = items
                .into_iter()
                .map(|row| {
                    let id = Some(row.try_get("id").unwrap());
                    let data: serde_json::Value = row.try_get("data")?;
                    let data: HashMap<String, serde_json::Value> = serde_json::from_value(data)?;
                    Ok(ContentItem { id, data })
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

pub async fn get_content_item(
    State(state): State<AppState>,
    Path((_service_id, content_item_id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    let row = sqlx::query(
        "SELECT id, data FROM content_items WHERE id = $1"
    )
        .bind(content_item_id)
        .fetch_one(&state.postgres)
        .await;

    match row {
        Ok(row) => {
            let id = Some(row.try_get("id").unwrap());
            let data: serde_json::Value = row.try_get("data").unwrap();
            let data: HashMap<String, serde_json::Value> = serde_json::from_value(data).unwrap();
            let content_item = ContentItem { id, data };
            Json(content_item).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの取得に失敗しました: {}", e),
        )
            .into_response(),
    }
}
