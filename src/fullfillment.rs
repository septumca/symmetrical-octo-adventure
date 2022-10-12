use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize};

use crate::{DbState, error::{self}};

pub async fn create(
  Json(payload): Json<CreateFullfillment>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let CreateFullfillment { requirement, user } = payload;
  let _ = sqlx::query!(
      r#"
  INSERT INTO fullfillment ( requirement, user )
  VALUES ( ?1, ?2 )
      "#,
      requirement, user
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  Ok(())
}

pub async fn delete(
  Path((user_id, requirement_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM fullfillment
  WHERE user = ?1 AND requirement = ?2
      "#,
      user_id, requirement_id
    )
    .execute(&pool)
    .await?;

  Ok(())
}


#[derive(Deserialize)]
pub struct CreateFullfillment {
  requirement: i64,
  user: i64,
}
