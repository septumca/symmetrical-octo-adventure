use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;

pub enum AppError {
  DB(String),
  Server(String),
  NotFound(String),
  Unauthorized(String)
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let (status, error_message) = match self {
      AppError::Unauthorized(msg) => {
        (
          StatusCode::NOT_FOUND,
          format!("{} - {}", "Unauthorized", msg)
        )
      },
      AppError::Server(msg) => {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          format!("{} - {}", "Internal Server Error", msg)
        )
      }
      AppError::DB(msg) => {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          format!("{} - {}", "Database Error", msg)
        )
      }
      AppError::NotFound(msg) => {
        (
          StatusCode::NOT_FOUND,
          format!("{} - {}", "Not Found", msg)
        )
      }
    };
    let body = Json(json!({
      "error": error_message,
    }));

    (status, body).into_response()
  }
}

impl From<axum::Error> for AppError {
  fn from(e: axum::Error) -> Self {
    AppError::Server(e.to_string())
  }
}

impl From<std::io::Error> for AppError {
  fn from(e: std::io::Error) -> Self {
    AppError::Server(e.to_string())
  }
}

impl From<sqlx::Error> for AppError {
  fn from(e: sqlx::Error) -> Self {
    AppError::DB(e.to_string())
  }
}
