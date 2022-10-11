use axum::{
  http::Method,
  routing::{get, post, put, delete},
  Router, Extension, middleware,
};
use dotenvy::dotenv;
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::{shutdown_signal};
use user::{authentificate};
use std::{env};
use std::net::SocketAddr;
use sqlx::{SqlitePool, Pool, Sqlite};
use db_modeling::{database_down, database_up};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower::ServiceBuilder;

mod error;
mod utils;
mod auth;
mod db_modeling;
mod user;
mod event;
mod requirement;
mod fullfillment;

type DbState = Pool<Sqlite>;

#[allow(dead_code)]
async fn app(pool: Pool<Sqlite>) -> Router {
  let cors = CorsLayer::new()
  .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
  .allow_headers(Any)
  .allow_origin(Any);

  let public = Router::new()
    .route("/up", get(database_up))
    .route("/down", get(database_down))
    .route("/register", post(user::create))
    .route("/authentificate", post(authentificate))
    ;

  let private = Router::new()
    .route("/user/:id", get(user::single))
    .route("/user/:id", put(user::update))
    .route("/user/:id", delete(user::delete))
    .route("/user", get(user::all))

    .route("/event", post(event::create))
    .route("/event/:id", get(event::single))
    .route("/event/:id", put(event::update))
    .route("/event/:id", delete(event::delete))
    .route("/event", get(event::all))

    .route("/requirement", post(requirement::create))
    .route("/requirement/:id", put(requirement::update))
    .route("/requirement/:id", delete(requirement::delete))

    .route("/fullfillment", post(fullfillment::create))
    .route("/fullfillment/:id", delete(fullfillment::delete))

    .route_layer(middleware::from_fn(auth::auth))
    ;

  Router::new()
    .merge(public)
    .merge(private)
    .layer(
      ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(Extension(cors))
        .layer(Extension(pool))
    )
}



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
  let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
  tracing::info!("listening on {}", addr);
  axum::Server::bind(&addr)
    .serve(app(pool).await.into_make_service())
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

// https://github.com/tokio-rs/axum/tree/0.5.x/examples/testing

#[cfg(test)]
mod tests {
  use crate::utils::read_file;

  use super::*;
  use axum::{
      body::Body,
      http::{self, Request, StatusCode},
  };
  use serde_json::{json, Value};
  use std::net::{SocketAddr, TcpListener};
  use tower::ServiceExt; // for `app.oneshot()`

  async fn setup(init_db: bool) -> Router {
    env::set_var("DATABASE_URL", "sqlite::memory:");
    let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();
    if init_db {
      let sql = read_file("up.sql").unwrap();
      sqlx::query(&sql).execute(&pool).await.unwrap();
    }
    app(pool).await
  }

  #[tokio::test]
  async fn db_up() {
    let app = setup(false).await;

    let response = app
        .oneshot(
            Request::builder()
              .method(http::Method::GET)
              .uri("/up")
              .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
              .body(Body::empty())
              .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
  }

  #[tokio::test]
  async fn db_down() {
    let app = setup(true).await;

    let response = app
        .oneshot(
            Request::builder()
              .method(http::Method::GET)
              .uri("/down")
              .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
              .body(Body::empty())
              .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
  }

  mod user {
    use super::*;

    #[tokio::test]
    async fn register() {
      let app = setup(true).await;
      let body_json = json!({
        "username": "Janko Hrasko",
        "password": "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
      });
      let response_json = json!({
        "id": 1,
        "username": "Janko Hrasko"
      });

      let response = app
          .oneshot(
              Request::builder()
                .method(http::Method::POST)
                .uri("/register")
                .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .body(Body::from(
                    serde_json::to_vec(&body_json).unwrap(),
                ))
                .unwrap(),
          )
          .await
          .unwrap();

      assert_eq!(response.status(), StatusCode::OK);

      let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
      let body: Value = serde_json::from_slice(&body).unwrap();
      assert_eq!(body, response_json);
    }
  }

}