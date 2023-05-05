use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};

use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;
use sqlx::{Row};
use anyhow::Result;
use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterDetails {
    username: String,
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct LoginDetails {
    username: String,
    password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(new_user): Json<RegisterDetails>,
) -> impl IntoResponse {
    //空パスワードを回避する。ログイン時はハッシュ化されたパスワードを検証。
    let hashed_password = bcrypt::hash(new_user.password, 10).unwrap();
    let query = sqlx::query("INSERT INTO users (username, email, password) values ($1, $2, $3)")
        .bind(new_user.username)
        .bind(new_user.email)
        .bind(hashed_password)
        .execute(&state.postgres);

    //作成成功したら Created status code, 失敗したら Internal Server Error status code を返す。
    match query.await {
        Ok(_) => (StatusCode::CREATED, "作成されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR, format!("作成できませんでした: {}", e)
        ).into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(login): Json<LoginDetails>,
) -> Result<(PrivateCookieJar, StatusCode), StatusCode> {
    let query = sqlx::query("SELECT * FROM users WHERE username = $1")
        .bind(login.username)
        .fetch_one(&state.postgres);
    match query.await {
        Ok(res) => {

            //bcryptがハッシュ値を認証できなかったら、BAD_REQUESTエラーを返す。
            if bcrypt::verify(login.password, res.get("password")).is_err() {
                return Err(StatusCode::BAD_REQUEST);
            }
            //ランダムセッションIDを生成し、ハッシュマップエントリーに追加
            let session_id = rand::random::<u64>().to_string();

            sqlx::query("INSERT INTO sessions (session_id, user_id) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET session_id = EXCLUDED.session_id")
                .bind(&session_id)
                .bind(res.get::<i32, _>("id"))
                .execute(&state.postgres)
                .await
                .expect("Couldn't insert session :(");

            let cookie = Cookie::build("foo", session_id)
                .secure(true)
                .same_site(SameSite::Strict)
                .http_only(true)
                .path("/")
                .finish();

            //ステータスコード200とクッキーを返す。
            Ok((jar.add(cookie), StatusCode::OK))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn logout(
    State(state): State<AppState>,
    jar: PrivateCookieJar)
    -> Result<PrivateCookieJar, StatusCode> {
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {
        return Ok(jar);
    };

    let query = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
        .bind(cookie)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => Ok(jar.remove(Cookie::named("foo"))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(email_recipient): Json<String>,
) -> Response {
    let new_password = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let hashed_password = bcrypt::hash(&new_password, 10).unwrap();

    sqlx::query("UPDATE users SET password = $1 WHERE email = $2")
        .bind(&hashed_password)
        .bind(&email_recipient)
        .execute(&state.postgres)
        .await.expect("Couldn't update password");

    let credentials = Credentials::new(state.smtp_email, state.smtp_password);

    let message = format!("Hello! \n\n Your new password is: {}", new_password);

    let email = Message::builder()
        .from("no reply".parse().unwrap())
        .to(format!("<{email_recipient}>").parse().unwrap())
        .subject("Forgot Password")
        .header(ContentType::TEXT_PLAIN)
        .body(message)
        .unwrap();

    //メールを送信する
    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(credentials)
        .build();

    match mailer.send(&email) {
        Ok(_) => (StatusCode::OK, "Sent".to_string()).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Error: {e}")).into_response(),
    }
}
