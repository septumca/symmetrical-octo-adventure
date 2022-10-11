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

  let cors = CorsLayer::new()
    .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
    .allow_headers(Any)
    .allow_origin(Any);

  let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();

  let public = Router::new()
    .route("/up", get(database_up))
    .route("/down", get(database_down))
    .route("/register", post(user::create))
    .route("/authentificate", post(authentificate))
    ;

  let private = Router::new()
    .route("/hello", get(hello))
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

  let app = Router::new()
      .merge(public)
      .merge(private)
      .layer(
        ServiceBuilder::new()
          .layer(TraceLayer::new_for_http())
          .layer(Extension(cors))
          .layer(Extension(pool))
      );

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
