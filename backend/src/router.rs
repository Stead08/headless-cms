use crate::router_comp::{
    auth_router::{forgot_password, login, logout, register},
    content_router::{create_content_item, create_content_type, create_field, get_content_items},
    service_router::{create_service, delete_service},
};
use crate::AppState;

use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::cookie::PrivateCookieJar;
use http::{
    header::{ACCEPT, AUTHORIZATION, ORIGIN},
    HeaderValue, Method,
};

use tower_http::cors::CorsLayer;

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
        .route("/fields", post(create_field))
        .route("/content_items", post(create_content_item))
        .route("/content_items/:content_type_id", get(get_content_items))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            validate_session,
        ));
    let create_service = Router::new().route("/", post(create_service));
    let service_router = Router::new()
        .route("/services/:service_id", delete(delete_service))
        .nest("/content", content_router)
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            validate_session,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            validate_api_key,
        ));


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

async fn validate_api_key<B>(
    State(state): State<AppState>,
    // you can add more extractors here but the last
    // extractor must implement `FromRequest` which
    // `Request` does
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
    let find_api_key = sqlx::query("SELECT api_key FROM services WHERE api_key = $1")
        .bind(api_key)
        .fetch_one(&state.postgres)
        .await;

    match find_api_key {
        Ok(_) => next.run(request).await,
        Err(_) => (StatusCode::UNAUTHORIZED, "API-KEYが異なります".to_string()).into_response(),
    }
}
