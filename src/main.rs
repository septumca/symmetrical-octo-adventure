use axum::{
  http::Method,
  routing::{get, post, put, delete},
  Router, Extension,
};
use dotenvy::dotenv;
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::{shutdown_signal};
use user::{authentificate};
use std::{env};
use std::net::SocketAddr;
use sqlx::{SqlitePool, Pool, Sqlite};
use db_modeling::{database_down, database_up, database_fill};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower::ServiceBuilder;

mod error;
mod utils;
mod auth;
mod db_modeling;
mod user;
mod event;
mod participant;
mod requirement;
mod fullfillment;

type DbState = Pool<Sqlite>;

pub async fn app(pool: Pool<Sqlite>) -> Router {
  let cors = CorsLayer::new()
    .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
    .allow_headers(Any)
    .allow_origin(Any);

  let public = Router::new()
    .route("/up", get(database_up))
    .route("/down", get(database_down))
    .route("/fill", get(database_fill))
    .route("/verify_captcha", post(auth::verify_captcha))
    .route("/register", post(user::create))
    .route("/authentificate", post(authentificate))

    .route("/event", get(event::all))
    .route("/event/:id", get(event::single))
    .route("/event", post(event::create))
    .route("/event/:id", put(event::update))
    .route("/event/:id", delete(event::delete))

    // .route("/user", get(user::all))
    .route("/user/:id", get(user::single))
    .route("/user/:id", put(user::update))
    .route("/user/:id", delete(user::delete))

    .route("/participant", post(participant::create))
    .route("/participant/:user_id/:event_id", delete(participant::delete))

    .route("/requirement", post(requirement::create))
    .route("/requirement/:id", put(requirement::update))
    .route("/requirement/:id", delete(requirement::delete))

    .route("/fullfillment", post(fullfillment::create))
    .route("/fullfillment/:user_id/:requirement_id", delete(fullfillment::delete))
    ;

  public
    .layer(
      ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
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
