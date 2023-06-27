use crate::models::sessions;
use crate::router_comp::content_router::update_content_item;
use crate::router_comp::{
    auth_router::{forgot_password, login, logout, register},
    content_router::{
        create_content_item, create_content_type, create_field, delete_content_item,
        get_content_item, get_content_items, get_content_type,
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

use http::header::CONTENT_TYPE;
use http::{
    header::{ACCEPT, AUTHORIZATION, ORIGIN},
    HeaderValue, Method,
};
use hyper::{Body, Client};
use hyper::body::to_bytes;
use hyper_tls::HttpsConnector;
use jsonwebtoken::{Algorithm, decode, decode_header, DecodingKey, Validation};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use serde::Deserialize;
use serde_json::Value;

use crate::router_comp::auth_router::auth_check;
use tower::limit::RateLimitLayer;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use uuid::Uuid;

pub fn create_router(state: AppState) -> Router {
    let api_router = api_router(state);
    let dir_router = Router::new().nest_service("/", ServeDir::new("../frontend/out"));

    //API ルーターを「/api」ルートにネスト。
    Router::new().nest("/", dir_router).nest("/api", api_router)
}

pub fn api_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_credentials(true)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(vec![ACCEPT, AUTHORIZATION, ORIGIN, CONTENT_TYPE])
        .allow_origin(state.domain.parse::<HeaderValue>().unwrap());

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
        .route_layer(middleware::from_fn_with_state(state.clone(), validate_session));

    let service_router = Router::new()
        .route("/services/:service_id", delete(delete_service))
        .nest("/:service_id", content_router);

    Router::new()
        .route("/health", get(health_check))
        .nest("/service", create_service)
        .nest("/services", service_router)
        .with_state(state)
        .layer(cors)
}


pub async fn health_check() -> Response {
    (StatusCode::OK, "OK!").into_response()
}

pub async fn validate_session<B>(
    State(state): State<AppState>,
    // Request<B> と Next<B> は axum の関数からのミドルウェアに必要な型
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let jwks = &state.jwks;
    //AUTHORIZATION ヘッダを取得
    let Some(authorization_header) = request.headers().get("AUTHORIZATION") else {
        return (StatusCode::UNAUTHORIZED, "no authorization header".to_string()).into_response() };

    let Ok(authorization) = authorization_header.to_str() else { return StatusCode::UNAUTHORIZED.into_response() };

    // jwt tokenだけ剥がす
    let Some(jwt_token) = authorization.strip_prefix("Bearer ") else {
        return (StatusCode::UNAUTHORIZED, "No Bearer".to_string()).into_response() };
    // tokenをdecodeする
    let Ok(header) = decode_header(jwt_token) else { return (StatusCode::UNAUTHORIZED, "failed to decode header".to_string()).into_response() };
    //kidを取得
    let Some(kid) = header.kid else { return (StatusCode::UNAUTHORIZED, "no valied kid".to_string()).into_response() };
    //kidに対応するjwkを取得
    let Some(jwk) = jwks.find(kid.as_str()) else { return (StatusCode::UNAUTHORIZED, "no valid jwk".to_string()).into_response() };
    // jwkからDecodingKeyを生成
    let decoding_key = DecodingKey::from_jwk(jwk).expect("failed to decode key");
    // RS256を指定
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[&state.audience]);
    validation.set_issuer(&[&state.issuer]);

    match decode::<Value>(jwt_token, &decoding_key, &validation) {
        // JWTのデコードと検証を行う
        Ok(_value) => {
            //auth0にユーザ情報の問い合わせを行う
            let userinfo_uri = format!("{}{}", state.issuer, "userinfo");

            let https = HttpsConnector::new();
            let client = Client::builder().build::<_, Body>(https);
            let req = Request::builder()
                .method(Method::GET)
                .uri(userinfo_uri)
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {}", jwt_token))
                .body(Body::empty()).expect("failed to build request");
            let response = client.request(req).await.expect("failed to fetch userinfo");
            let body_bytes = to_bytes(response.into_body()).await.expect("failed to read body");
            let json_body: Value = serde_json::from_slice(&body_bytes).unwrap();
            eprintln!("{:?}", json_body);
            next.run(request).await},
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
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
                    Ok(_) => next.run(request).await, // 許可されている
                    Err(_) => (
                        StatusCode::FORBIDDEN,
                        "メソッドが許可されていません".to_string(),
                    )
                        .into_response(), // 許可されていない
                }
            } else {
                (StatusCode::FORBIDDEN).into_response()
            }
        }
        Err(_) => (StatusCode::BAD_REQUEST).into_response(),
    }
}
