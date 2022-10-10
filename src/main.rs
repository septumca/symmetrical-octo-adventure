use auth::{generate_salt, get_salted_password, generate_jwt};
use axum::{
    routing::{get, post},
    Json, Router, Extension, middleware,
};
use dotenvy::dotenv;
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::{read_file, shutdown_signal};
use std::{env};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use sqlx::{SqlitePool, Pool, Sqlite};

mod error;
mod utils;
mod auth;

type DbState = Pool<Sqlite>;


#[tokio::main]
async fn main() {
  dotenv().ok();
  let filter = EnvFilter::from_default_env();
  let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);
  tracing_subscriber::registry()
    .with(filtered_layer)
    .init();

  for (key, value) in env::vars() {
    tracing::debug!("{key}: {value}");
  }

  let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();

  let public = Router::new()
    .route("/up", get(database_up))
    .route("/down", get(database_down))
    .route("/register", post(create_user))
    .route("/authentificate", post(authentificate));

  let private = Router::new()
    .route("/hello", get(hello))
    .route_layer(middleware::from_fn(auth::auth));

  let app = Router::new()
      .merge(public)
      .merge(private)
      .layer(Extension(pool));

  let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
  tracing::info!("listening on {}", addr);
  axum::Server::bind(&addr)
    .serve(app.into_make_service())
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}



async fn hello() -> &'static str {
  "Hello, World!"
}

async fn database_up(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  let sql = read_file("up.sql")?;
  sqlx::query(&sql).execute(&pool).await?;

  Ok(())
}

async fn database_down(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  let sql = read_file("down.sql")?;
  sqlx::query(&sql).execute(&pool).await?;
  Ok(())
}

async fn create_user(
  Json(payload): Json<CreateUser>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<User>, error::AppError> {
  let salt = generate_salt();
  let password = get_salted_password(&payload.password, &salt.clone());

  let id = sqlx::query!(
      r#"
  INSERT INTO user ( username, password, salt )
  VALUES ( ?1, ?2, ?3 )
      "#,
      payload.username, password, salt
  )
  .execute(&pool)
  .await?
  .last_insert_rowid();

  let user = User {
    id,
    username: payload.username,
  };

  Ok(Json(user))
}

pub async fn authentificate(
  Json(data): Json<UserAuthReqData>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<UserAuthRespData>, error::AppError> {
  let user_db = sqlx::query!(
      "
      SELECT id, password, salt
      FROM user
      WHERE username = ?
      ",
      data.username
  )
  .fetch_one(&pool)
  .await?;

  let calculated_password = get_salted_password(&data.password, &user_db.salt);
  if calculated_password != user_db.password {
    return Err(error::AppError::Unauthorized(String::from("incorrect password")));
  }

  let resp = UserAuthRespData {
    id: 0,
    token: generate_jwt(format!("{}", user_db.id))
  };
  Ok::<Json<UserAuthRespData>, error::AppError>(Json(resp))

}

#[derive(Deserialize)]
struct CreateUser {
  username: String,
  password: String,
}

#[derive(Serialize)]
struct User {
  id: i64,
  username: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserAuthReqData {
  username: String,
  password: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct UserAuthRespData {
  id: i64,
  token: String,
}