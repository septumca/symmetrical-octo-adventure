use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, utils::AppReponse, error::AppError, auth::UserAuth};

pub async fn create(
  Json(payload): Json<CreateParticipant>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<CreateParticipantResponse>> {
  let CreateParticipant { event, user } = payload;
  if user != auth_userid {
    return Err(AppError::Unauthorized(format!("cannot make participation for another user, participting user: {}, requester: {}", user, auth_userid)));
  }

  let selected_user = sqlx::query!(
      r#"
  SELECT username FROM user WHERE id = ?1
      "#,
      user
    )
    .fetch_one(&pool)
    .await?;

  let _ = sqlx::query!(
      r#"
  INSERT INTO participant ( event, user )
  VALUES ( ?1, ?2 )
      "#,
      event, user
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let participant = CreateParticipantResponse {
    user,
    username: selected_user.username
  };
  Ok((StatusCode::CREATED, Json(participant)))
}

pub async fn delete(
  Path((user_id, event_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM participant
  WHERE user = ?1 AND event = ?2
      "#,
      user_id, event_id
    )
    .execute(&pool)
    .await?;

  Ok((StatusCode::NO_CONTENT, ()))
}

#[derive(Deserialize)]
pub struct CreateParticipant {
  event: i64,
  user: i64,
}


#[derive(Serialize)]
pub struct CreateParticipantResponse {
  username: String,
  user: i64,
}
