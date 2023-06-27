use crate::models::prelude::{Sessions, Users};
use crate::models::sessions::ActiveModel as SessionModel;
use crate::models::sessions::Entity as SessionEntity;
use crate::models::users::ActiveModel as UserModel;
use crate::models::{sessions, users};
use crate::{models, AppState};
use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::distributions::{Alphanumeric, DistString};
use sea_orm::prelude::*;
use sea_orm::ActiveValue::Set;
use sea_orm::{sea_query, IntoActiveModel, NotSet};
use serde::Deserialize;
use time::Duration;

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

    let user = UserModel {
        id: Default::default(),
        username: Set(new_user.username),
        email: Set(new_user.email),
        password: Set(hashed_password),
        createdat: NotSet,
    };

    let res = user.insert(&state.postgres);

    //作成成功したら Created status code, 失敗したら Internal Server Error status code を返す。
    match res.await {
        Ok(_) => (StatusCode::CREATED, "作成されました".to_string()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("作成できませんでした: {}", e),
        )
            .into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(login): Json<LoginDetails>,
) -> Result<(PrivateCookieJar, StatusCode), StatusCode> {
    let user = Users::find()
        .filter(users::Column::Username.eq(&login.username))
        .one(&state.postgres)
        .await;

    match user {
        Ok(Some(user)) => {
            // bcryptがハッシュ値を認証できなかったら、BAD_REQUESTエラーを返す。
            match bcrypt::verify(&login.password, &user.password) {
                Ok(bool) => {
                    if !bool {
                        return Err(StatusCode::UNAUTHORIZED);
                    }
                }
                Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
            }

            // ランダムセッションIDを生成し、ハッシュマップエントリーに追加
            let session_id = rand::random::<u64>().to_string();

            let session = SessionModel {
                id: Default::default(),
                session_id: Set(session_id.clone()),
                user_id: Set(user.id),
            };

            let result = SessionEntity::insert(session)
                .on_conflict(
                    // on conflict do update
                    sea_query::OnConflict::column(models::sessions::Column::UserId)
                        .update_column(models::sessions::Column::SessionId)
                        .to_owned(),
                )
                .exec(&state.postgres);

            match result.await {
                Ok(_) => {
                    let cookie = Cookie::build("foo", session_id)
                        .secure(false)
                        .same_site(SameSite::Lax)
                        .http_only(true)
                        .path("/")
                        .max_age(Duration::WEEK)
                        .finish();
                    // ステータスコード200とクッキーを返す。
                    Ok((jar.add(cookie), StatusCode::OK))
                }
                Err(e) => {
                    eprintln!("An error occurred: {:?}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        Ok(None) => Err(StatusCode::BAD_REQUEST),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn logout(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> Result<PrivateCookieJar, StatusCode> {
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {
        return Ok(jar);
    };

    //削除対象を取得する
    let target = models::sessions::Entity::find()
        .filter(models::sessions::Column::SessionId.eq(cookie))
        .one(&state.postgres)
        .await;

    match target {
        Ok(target) => {
            //ActiveModelを取得する
            let delete_row: models::sessions::ActiveModel = target.unwrap().into_active_model();
            let delete_result = delete_row.delete(&state.postgres).await;
            match delete_result {
                Ok(_) => Ok(jar.remove(Cookie::named("foo"))),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(email_recipient): Json<String>,
) -> Response {
    let new_password = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
    let hashed_password = bcrypt::hash(&new_password, 10).unwrap();

    //更新対象を取得する
    let target = users::Entity::find()
        .filter(users::Column::Email.eq(&email_recipient))
        .one(&state.postgres)
        .await;

    //パスワードを更新する
    match target {
        Ok(target) => {
            //ActiveModelを取得する
            let mut update_row: users::ActiveModel = target
                .expect("アクティブモデルへの変換失敗")
                .into_active_model();
            //hashed_passwordに更新する
            update_row.password = Set(hashed_password);
            let update_result = update_row.update(&state.postgres).await;
            match update_result {
                Ok(_) => {
                    let credentials =
                        Credentials::new(state.smtp_email.clone(), state.smtp_password);

                    let message = format!("Hello! \n\n Your new password is: {}", new_password);

                    let email = Message::builder()
                        .from(state.smtp_email.parse().expect("failed to parse from"))
                        .to(format!("<{email_recipient}>")
                            .parse()
                            .expect("failed to parse to"))
                        .subject("Forgot Password")
                        .header(ContentType::TEXT_PLAIN)
                        .body(message)
                        .unwrap();

                    //メールを送信する
                    let mailer = SmtpTransport::relay("smtp.mail.yahoo.co.jp")
                        .unwrap()
                        .credentials(credentials)
                        .build();

                    match mailer.send(&email) {
                        Ok(_) => {
                            (StatusCode::OK, "メールを送信しました".to_string()).into_response()
                        }
                        Err(e) => {
                            eprintln!("{}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "メールを送信できませんでした".to_string(),
                            )
                                .into_response()
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "パスワードを更新できませんでした".to_string(),
                    )
                        .into_response()
                }
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            "メールアドレスが見つかりませんでした".to_string(),
        )
            .into_response(),
    }
}

pub async fn auth_check(State(state): State<AppState>, jar: PrivateCookieJar) -> impl IntoResponse {
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {
        println!("{:?} Could not find a cookie in jar", jar);
        return (StatusCode::FORBIDDEN, "ログインしてください".to_string()).into_response();
    };

    let find_session = Sessions::find()
        .filter(sessions::Column::SessionId.eq(cookie))
        .one(&state.postgres)
        .await;

    match find_session {
        Ok(session) => match session {
            Some(_) => (StatusCode::OK, "ログインしています".to_string()).into_response(),
            None => (StatusCode::FORBIDDEN, "ログインしていません".to_string()).into_response(),
        },
        Err(_) => (StatusCode::FORBIDDEN, "ログインしていません".to_string()).into_response(),
    }
}

// #[cfg(test)]
// mod tests {
//     use std::time::Duration;
//     use anyhow::Error;
//     use axum_extra::extract::cookie::Key;
//
//     use sea_orm::SqlxPostgresConnector;
//     use sqlx::postgres::PgPoolOptions;
//
//     use super::*;
//
//     async fn create_state() -> Result<AppState, Error> {
//         //接続文字列
//         let db_address = dotenv::var("DATABASE_URL").expect("DATABASE_URL must be set");
//         let pgpool = PgPoolOptions::new()
//             .max_connections(5)
//             .idle_timeout(Some(Duration::from_secs(1)))
//             .connect(&db_address)
//             .await?;
//
//         let conn = SqlxPostgresConnector::from_sqlx_postgres_pool(pgpool.clone());
//
//         let state = AppState {
//             postgres: conn,
//             pgpool,
//             key: Key::generate(),
//             smtp_email: "".to_string(),
//             smtp_password: "".to_string(),
//             domain: "".to_string(),
//         };
//         Ok(state)
//     }
//
//     //データベース初期化
//     async fn init_db() -> Result<(), Error> {
//         let db_address = dotenv::var("DATABASE_URL").expect("DATABASE_URL must be set");
//         let pgpool = PgPoolOptions::new()
//             .max_connections(5)
//             .idle_timeout(Some(Duration::from_secs(1)))
//             .connect(&db_address)
//             .await
//             .expect("failed to connect to postgres");
//         //テーブル初期化
//         sqlx::query("DROP TABLE IF EXISTS sessions CASCADE;")
//             .execute(&pgpool)
//             .await
//             .expect("failed to drop sessions table");
//         // Delete table and sequence
//         sqlx::query("DROP TABLE IF EXISTS users CASCADE;")
//             .execute(&pgpool)
//             .await
//             .expect("failed to drop users table");
//
//         sqlx::query(
//             r#"
//     CREATE TABLE IF NOT EXISTS users
//     (
//         id        SERIAL PRIMARY KEY,
//         username  VARCHAR UNIQUE NOT NULL,
//         email     VARCHAR UNIQUE NOT NULL,
//         password  VARCHAR        NOT NULL,
//         createdAt TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
//     );
//     "#,
//         )
//             .execute(&pgpool)
//             .await
//             .expect("failed to create users table");
//
//         sqlx::query(
//             r#"
//     CREATE TABLE IF NOT EXISTS sessions
//     (
//         id         SERIAL PRIMARY KEY,
//         session_id VARCHAR NOT NULL UNIQUE,
//         user_id    INT     NOT NULL UNIQUE REFERENCES users(id)
//     );
//     "#,
//         )
//             .execute(&pgpool)
//             .await
//             .expect("failed to create sessions table");
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_register() {
//         init_db().await.expect("failed to initialize database");
//         //テストユーザーを登録
//         let state = create_state().await.expect("failed to create state");
//         let test_user = RegisterDetails {
//             username: "test".to_string(),
//             email: "test@example.com".to_string(),
//             password: "password".to_string(),
//         };
//         let response = register(State(state), Json(test_user)).await.into_response();
//         //ステータスコードが201であることを確認
//         assert_eq!(response.status(), StatusCode::CREATED);
//
//         //ティアダウン
//         init_db().await.expect("failed to initialize database");
//     }
//
//     #[tokio::test]
//     async fn test_login() {
//         init_db().await.expect("failed to initialize database");
//         //テストユーザーを登録
//         let state = create_state().await.expect("failed to create state");
//         let test_user = RegisterDetails {
//             username: "test".to_string(),
//             email: "test@example.com".to_string(),
//             password: "password".to_string(),
//         };
//         let response = register(State(state.clone()), Json(test_user)).await.into_response();
//         //responseが201で処理を続ける
//         if response.status() != StatusCode::CREATED {
//             panic!("failed to register test user");
//         }
//         //ログイン
//         let login_details = LoginDetails {
//             username: "test".to_string(),
//             password: "password".to_string(),
//         };
//         let response = login(State(state.clone()), PrivateCookieJar::new(state.key), Json(login_details))
//             .await
//             .into_response();
//
//         //ステータスコードが200であることを確認
//         assert_eq!(response.status(), StatusCode::OK);
//
//         init_db().await.expect("failed to initialize database");
//     }
// }
//
