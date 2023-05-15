use crate::models::prelude::Sessions;
use crate::models::{sessions};
use crate::router_comp::content_router::update_content_item;
use crate::router_comp::{
    auth_router::{forgot_password, login, logout, register},
    content_router::{
        create_content_item, create_content_type, create_field, delete_content_item,
        get_content_items, get_content_item, get_content_type,
    },
    service_router::{create_role, create_service, delete_service},
};
use crate::{models, AppState};
use axum::extract::Path;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Router,
};
use axum_extra::extract::cookie::PrivateCookieJar;
use http::{
    header::{ACCEPT, AUTHORIZATION, ORIGIN},
    HeaderValue, Method,
};
use http::header::CONTENT_TYPE;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub fn create_router(state: AppState) -> Router {
    let api_router = api_router(state);

    //API ルーターを「/api」ルートにネスト。
    Router::new().nest("/api", api_router)
}

pub fn api_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(vec![ACCEPT, AUTHORIZATION, ORIGIN, CONTENT_TYPE])
        .allow_origin(state.domain.parse::<HeaderValue>().unwrap());

    let auth_router = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/forgot", post(forgot_password))
        .route("/logout", get(logout));

    let content_router = Router::new()
        .route("/content_types", post(create_content_type))
        .route("/content_types/:content_type_id", get(get_content_type))
        .route("/:content_type_id/fields", post(create_field))
        .route("/:content_type_id/content_items", post(create_content_item))
        .route("/:content_type_id/content_items", get(get_content_items))
        .route("/content_items/:content_item_id", get(get_content_item))
        .route(
            "/content_items/:content_item_id",
            patch(update_content_item),
        )
        .route(
            "/content_items/:content_item_id",
            delete(delete_content_item),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            validate_api_key,
        ));

    let create_service = Router::new()
        .route("/", post(create_service))
        .route("/:service_id/roles", post(create_role))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            validate_session,
        ));

    let service_router = Router::new()
        .route("/services/:service_id", delete(delete_service))
        .nest("/:service_id", content_router);

    Router::new()
        .nest("/auth", auth_router)
        .nest("/service", create_service)
        .nest("/services", service_router)
        .with_state(state)
        .layer(cors)
}

pub async fn validate_session<B>(
    jar: PrivateCookieJar,
    State(state): State<AppState>,
    // Request<B> と Next<B> は axum の関数からのミドルウェアに必要な型
    request: Request<B>,
    next: Next<B>,
) -> (PrivateCookieJar, Response) {
    //cookieの取得を試みる、できなかったら403を返す
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {
        println!("Could not find a cookie in jar");
        return (jar, (StatusCode::FORBIDDEN, "ログインしてください".to_string()).into_response());
    };

    //作成されたセッションを見つける
    let find_session = Sessions::find()
        .filter(sessions::Column::SessionId.eq(cookie))
        .one(&state.postgres)
        .await;
    //セッションが見つからなかったら、403を返す
    match find_session {
        Ok(_) => (jar, next.run(request).await),
        Err(_) => (
            jar,
            (StatusCode::FORBIDDEN, "ログインしてください".to_string()).into_response(),
        ),
    }
}

#[derive(Deserialize)]
pub struct PathParams {
    pub service_id: String,
    pub content_type_id: Option<i64>,
    pub content_item_id: Option<Uuid>,
}

async fn validate_api_key<B>(
    State(state): State<AppState>,
    Path(params): Path<PathParams>,
    request: Request<B>,
    next: Next<B>,
) -> Response {
    //requestからx-api-keyを見つけて取り出す
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();

    //作成されたAPIキーを見つける
    let find_service = models::prelude::Services::find_by_id(&params.service_id)
        .one(&state.postgres)
        .await;

    match find_service {
        Ok(service) => {
            //APIキーが一致したら、リクエストを次のミドルウェアに渡す
            if api_key == service.unwrap().api_key {
                // メソッドとサービスIDを使って、リクエストされたメソッドが許可されているか確認する

                let method = request.method();

                let has_permission = sqlx::query(
                    r#"SELECT EXISTS (
                    SELECT 1
                    FROM roles r
                    JOIN role_permissions rp ON r.id = rp.role_id
                    WHERE r.service_id = $1 AND rp.permission = $2
                )
                "#,
                )
                    .bind(params.service_id)
                    .bind(method.as_str())
                    .fetch_one(&state.pgpool)
                    .await;


                match has_permission {
                    Ok(_) => next.run(request).await,  // 許可されている
                    Err(_) => (
                        StatusCode::FORBIDDEN,
                        "メソッドが許可されていません".to_string(),
                    ).into_response(),  // 許可されていない
                }
            } else {
                (StatusCode::FORBIDDEN).into_response()
            }
        }
        Err(_) => (StatusCode::BAD_REQUEST).into_response(),
    }
}
