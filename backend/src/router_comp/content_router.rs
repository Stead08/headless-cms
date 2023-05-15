use crate::models::content_items::ActiveModel as ContentItemModel;
use crate::models::content_types::ActiveModel as ContentTypeModel;
use crate::models::fields::{ActiveModel as FieldModel, Model};
use crate::models::prelude::{ContentItems, ContentTypes, Fields};
use crate::models::{fields};
use crate::{models, AppState};
use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::postgres::PgTypeInfo;
use sqlx::{Decode, Type};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::{fmt, io};
use uuid::Uuid;

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

impl Model {
    // ModelからFieldへの変換
    pub fn to_field(&self) -> Field {
        Field {
            id: self.id,
            content_type_id: self.content_type_id,
            display_id: self.display_id.clone(),
            field_type: FieldType::from_str(&self.field_type).unwrap(),
            required: self.required,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    // field_type_matchesのラッパーメソッド
    pub fn field_type_matches(&self, value: &serde_json::Value) -> bool {
        self.to_field().field_type_matches(value)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Field {
    pub id: i32,
    pub content_type_id: i32,
    pub display_id: String,
    pub field_type: FieldType,
    pub required: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

impl Field {
    pub fn field_type_matches(&self, value: &serde_json::Value) -> bool {
        match self.field_type {
            FieldType::Text => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Date => {
                value.is_string() && {
                    let date_string = value.as_str().unwrap();
                    chrono::DateTime::parse_from_rfc3339(date_string).is_ok()
                }
            }
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

impl FromStr for FieldType {
    type Err = io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Text" => Ok(FieldType::Text),
            "Number" => Ok(FieldType::Number),
            "Date" => Ok(FieldType::Date),
            "Boolean" => Ok(FieldType::Boolean),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid field type",
            )),
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
    fn decode(
        value: <sqlx::Postgres as sqlx::database::HasValueRef<'r>>::ValueRef,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
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
    let content_type = ContentTypeModel {
        id: Default::default(),
        name: Set(new_content_type.name),
        created_at: Default::default(),
        updated_at: Default::default(),
        service_id: Set(Some(service_id)),
    };

    let res = content_type.insert(&state.postgres);

    match res.await {
        Ok(res) => {
            //問い合わせの返り値からidを取得
            let content_type_id: i32 = res.id;
            (
                StatusCode::CREATED,
                format!(
                    "コンテンツタイプが作成されました \n content_type_id: {}",
                    content_type_id
                ),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツタイプの作成に失敗しました: {}", e),
        )
            .into_response(),
    }
}

pub async fn create_field(
    State(state): State<AppState>,
    Path((_service_id, content_type_id)): Path<(String, i32)>,
    Json(new_field): Json<NewField>,
) -> impl IntoResponse {
    let new_field = FieldModel {
        id: Default::default(),
        content_type_id: Set(content_type_id),
        display_id: Set(new_field.display_name),
        field_type: Set(new_field.field_type.to_string()),
        required: Set(new_field.required),
        created_at: Default::default(),
        updated_at: Default::default(),
    };

    let query = new_field.insert(&state.postgres);

    match query.await {
        Ok(_) => (
            StatusCode::CREATED,
            "フィールドが作成されました".to_string(),
        )
            .into_response(),
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
    let content_type_row = ContentTypes::find_by_id(content_type_id).one(&state.postgres);

    match content_type_row.await {
        Ok(Some(row)) => {
            let fields_rows = Fields::find_by_id(content_type_id).all(&state.postgres);

            match fields_rows.await {
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
                        id: row.id,
                        name: row.name,
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
    let fields = Fields::find()
        .filter(fields::Column::ContentTypeId.eq(content_type_id))
        .all(&state.postgres)
        .await;

    match fields {
        Ok(fields) => {
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

            let content_item = ContentItemModel {
                id: Set(Uuid::new_v4()),
                content_type_id: Set(content_type_id),
                data: Set(new_content_item.data),
                created_at: Default::default(),
                updated_at: Default::default(),
            };

            let query = content_item.insert(&state.postgres);

            match query.await {
                Ok(_) => (
                    StatusCode::CREATED,
                    "コンテンツアイテムが作成されました".to_string(),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("コンテンツアイテムの作成に失敗しました: {}", e),
                )
                    .into_response(),
            }
        }
        Err(_) => (StatusCode::NOT_FOUND, "コンテンツタイプが見つかりません").into_response(),
    }
}

pub async fn delete_content_item(
    State(state): State<AppState>,
    Path((_service_id, content_item_id)): Path<(String, Uuid)>,
) -> impl IntoResponse {
    let query = ContentItems::delete_by_id(content_item_id).exec(&state.postgres);

    match query.await {
        Ok(_) => (
            StatusCode::OK,
            "コンテンツアイテムが削除されました".to_string(),
        )
            .into_response(),
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
    let query = ContentItems::find_by_id(content_item_id)
        .one(&state.postgres)
        .await;

    let content_type_id: i32 = match query {
        Ok(row) => match row {
            Some(row) => row.content_type_id,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    "コンテンツが存在しません".to_string(),
                )
                    .into_response()
            }
        },
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                "コンテンツが存在しません".to_string(),
            )
                .into_response();
        }
    };

    let query = Fields::find()
        .filter(fields::Column::ContentTypeId.eq(content_type_id))
        .all(&state.postgres)
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

    let target = ContentItems::find_by_id(content_item_id)
        .one(&state.postgres)
        .await;

    if target.is_err() {
        return (StatusCode::INTERNAL_SERVER_ERROR).into_response();
    }

    match target.unwrap() {
        Some(target) => {
            let mut update_row: ContentItemModel = target.into_active_model();
            update_row.data = Set(json_data);
            let update_result = update_row.update(&state.postgres).await;

            match update_result {
                Ok(_) => (
                    StatusCode::OK,
                    "コンテンツアイテムが更新されました".to_string(),
                )
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("コンテンツアイテムの更新に失敗しました: {}", e),
                )
                    .into_response(),
            }
        }
        None => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "no target found".to_string(),
            )
                .into_response()
        }
    }
}

pub async fn get_content_items(
    State(state): State<AppState>,
    Path((_service_id, content_type_id)): Path<(String, i32)>,
) -> impl IntoResponse {
    let rows = ContentItems::find()
        .filter(models::content_items::Column::ContentTypeId.eq(content_type_id))
        .all(&state.postgres)
        .await;

    match rows {
        Ok(items) => {
            if items.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    "コンテンツアイテムが見つかりませんでした".to_string(),
                )
                    .into_response();
            }

            let content_items: Result<Vec<ContentItem>, Box<dyn std::error::Error>> = items
                .into_iter()
                .map(|row| {
                    let id = Some(row.id);
                    let data: serde_json::Value = row.data;
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

    let row = ContentItems::find_by_id(content_item_id)
        .one(&state.postgres)
        .await;

    match row {
        Ok(row) => {
            match row {
                Some(row) => {
                    let id = row.id;
                    let data: serde_json::Value = row.data;
                    let data: HashMap<String, serde_json::Value> = serde_json::from_value(data).unwrap();
                    let content_item = ContentItem { id: Some(id), data };
                    Json(content_item).into_response()
                },
                None => (StatusCode::NOT_FOUND).into_response()
            }

        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("コンテンツアイテムの取得に失敗しました: {}", e),
        )
            .into_response(),
    }
}
