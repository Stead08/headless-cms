use crate::router_comp::{
    auth_router::{forgot_password, login, logout, register},
    content_router::{create_content_item, create_content_type, get_content_type, create_field, get_content_items, delete_content_item},
    service_router::{create_service, delete_service, create_role},
};
use crate::AppState;
use crate::router_comp::service_router::Service;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, patch},
    Router,
};
use axum::extract::Path;
use axum_extra::extract::cookie::PrivateCookieJar;
use serde::{Deserialize};
use http::{
    header::{ACCEPT, AUTHORIZATION, ORIGIN},
    HeaderValue, Method,
};

use tower_http::cors::CorsLayer;
use uuid::Uuid;
use crate::router_comp::content_router::update_content_item;


pub fn create_router(state: AppState) -> Router {
    let api_router = api_router(state);

    //API ルーターを「/api」ルートにネスト。
    Router::new().nest("/api", api_router)
}


pub fn api_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(vec![ACCEPT, AUTHORIZATION, ORIGIN])
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
        .route("/content_items/:content_item_id", patch(update_content_item))
        .route("/content_items/:content_item_id", delete(delete_content_item))
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
    let find_session = sqlx::query("SELECT * FROM sessions WHERE session_id = $1")
        .bind(cookie)
        .fetch_one(&state.postgres)
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
    let find_service = sqlx::query_as::<_, Service>("SELECT * FROM services WHERE id = $1")
        .bind(&params.service_id)
        .fetch_one(&state.postgres)
        .await;

    match find_service {
        Ok(service) => {
            //APIキーが一致したら、リクエストを次のミドルウェアに渡す
            if api_key == service.api_key {
                // メソッドとサービスIDを使って、リクエストされたメソッドが許可されているか確認する
                let method = request.method();
                let has_permission = sqlx::query(
                r#"
                SELECT EXISTS (
                    SELECT 1
                    FROM services_roles sr
                    JOIN role_permissions rp ON sr.role_id = rp.role_id
                    WHERE sr.service_id = $1 AND rp.permission = $2
                )
                "#)
                    .bind(params.service_id)
                    .bind(method.as_str())
                    .fetch_one(&state.postgres)
                    .await;

                match has_permission {
                    Ok(_) => next.run(request).await,
                    Err(_) => (StatusCode::FORBIDDEN, "メソッドが許可されていません".to_string()).into_response(),
                }
            } else {
                (StatusCode::FORBIDDEN).into_response()
            }},
        Err(_) => (StatusCode::BAD_REQUEST).into_response(),
    }
}
