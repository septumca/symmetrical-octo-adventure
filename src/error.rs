use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;

pub enum AppError {
  DB(String),
  Server(String),
  NotFound(String),
  Unauthorized(String),
  BadRequest(String),
  Forbidden(String),
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let (status, error_message) = match self {
      AppError::Unauthorized(msg) => {
        (
          StatusCode::UNAUTHORIZED,
          msg
        )
      },
      AppError::Server(msg) => {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          msg
        )
      }
      AppError::DB(msg) => {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          msg
        )
      }
      AppError::NotFound(msg) => {
        (
          StatusCode::NOT_FOUND,
          msg
        )
      },
      AppError::BadRequest(msg) => {
        (
          StatusCode::BAD_REQUEST,
          msg
        )
      },
      AppError::Forbidden(msg) => {
        (
          StatusCode::FORBIDDEN,
          msg
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

impl From<reqwest::Error> for AppError {
  fn from(e: reqwest::Error) -> Self {
    AppError::Server(e.to_string())
  }
}

impl From<serde_json::Error> for AppError {
  fn from(e: serde_json::Error) -> Self {
    AppError::Server(e.to_string())
  }
}


// pub struct AppError((StatusCode, String));

// impl IntoResponse for AppError {
//   fn into_response(self) -> Response {
//     let body = Json(json!({
//       "error": self.0.1,
//     }));

//     (self.0.0, body).into_response()
//   }
// }

// impl From<axum::Error> for AppError {
//   fn from(e: axum::Error) -> Self {
//     AppError((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//   }
// }

// impl From<std::io::Error> for AppError {
//   fn from(e: std::io::Error) -> Self {
//     AppError((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//   }
// }

// impl From<sqlx::Error> for AppError {
//   fn from(e: sqlx::Error) -> Self {
//     AppError((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//   }
// }

// impl From<reqwest::Error> for AppError {
//   fn from(e: reqwest::Error) -> Self {
//     AppError((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//   }
// }

// impl From<serde_json::Error> for AppError {
//   fn from(e: serde_json::Error) -> Self {
//     AppError((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
//   }
// }
