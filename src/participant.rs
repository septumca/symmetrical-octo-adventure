use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize};

use crate::{DbState, error::{self}};

pub async fn create(
  Json(payload): Json<CreateParticipant>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let CreateParticipant { event, user } = payload;
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

  Ok(())
}

pub async fn delete(
  Path((user_id, event_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM participant
  WHERE user = ?1 AND event = ?2
      "#,
      user_id, event_id
    )
    .execute(&pool)
    .await?;

  Ok(())
}

#[derive(Deserialize)]
pub struct CreateParticipant {
  event: i64,
  user: i64,
}
