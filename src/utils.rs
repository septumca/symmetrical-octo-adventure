use hyper::StatusCode;
use tokio::signal;

use crate::error;

pub type AppReponse<T> = Result<(StatusCode, T), error::AppError>;

pub async fn shutdown_signal() {
  let ctrl_c = async {
    signal::ctrl_c()
      .await
      .expect("failed to install Ctrl+C handler");
  };

  #[cfg(unix)]
  let terminate = async {
    signal::unix::signal(signal::unix::SignalKind::terminate())
      .expect("failed to install signal handler")
      .recv()
      .await;
  };

  #[cfg(not(unix))]
  let terminate = std::future::pending::<()>();

  tokio::select! {
    _ = ctrl_c => {},
    _ = terminate => {},
  }

  tracing::info!("signal received, starting graceful shutdown");
}

#[cfg(test)]
pub mod test {
  use crate::{auth::generate_jwt, app};
  use axum::{
    http::Method,
    body::Body,
    http::{self, Request, StatusCode, HeaderValue}, Router,
  };
  use serde_json::{Value};
  use tower::ServiceExt; // for `app.oneshot()`
  use std::{env};
  use sqlx::{SqlitePool};


  pub async fn test_api(app: Router, uri: &str, method: Method, body: Option<Value>, expected_status: StatusCode, auth: Option<(&str, &str)>) -> Option<Value> {
    let body = body.and_then(|b| Some(Body::from(serde_json::to_vec(&b).unwrap()))).unwrap_or(Body::empty());
    let mut req = Request::builder()
      .method(method)
      .uri(uri)
      .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref());
    if let Some((user_id, username)) = auth {
      let token = generate_jwt(user_id, username);
      let headers = req.headers_mut().unwrap();
      headers.insert("X-JWT-Token", HeaderValue::from_str(&token).unwrap());
    }

    let response = app
      .oneshot(
          req.body(body)
            .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), expected_status);

    let body: Result<Value, Box<dyn std::error::Error>> = async {
      let body = hyper::body::to_bytes(response.into_body()).await?;
      let body: Value = serde_json::from_slice(&body)?;
      Ok(body)
    }.await;

    body.ok()
  }

  pub async fn setup() -> (Router, SqlitePool) {
    env::set_var("DATABASE_URL", "sqlite::memory:");
    env::set_var("JWT_SECRET", "test-jwt-secret");
    let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();
    (app(pool.clone()).await, pool)
  }

  pub async fn setup_with_structure() -> (Router, SqlitePool) {
    let (app, pool) = setup().await;
    let _ = sqlx::query_file!("./sql/up.sql").execute(&pool).await.unwrap();
    (app, pool)
  }

  pub async fn setup_with_data() -> (Router, SqlitePool) {
    let (app, pool) = setup_with_structure().await;
    let _ = sqlx::query_file!("./sql/test.sql").execute(&pool).await.unwrap();
    (app, pool)
  }
}