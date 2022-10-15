use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize};

use crate::{DbState, error::{AppError}, utils::AppReponse};

pub async fn create(
  Json(payload): Json<CreateFullfillment>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<()> {
  let CreateFullfillment { requirement, user } = payload;
  let maximum = sqlx::query!(
    r#"
SELECT size FROM requirement WHERE id = ?1
    "#,
    requirement
  )
  .fetch_optional(&pool)
  .await?;

  if maximum.is_none() {
    return Err(AppError::NotFound(format!("Cannot find requirement: {requirement}")))
  }
  let maximum = maximum.unwrap();
  let existing = sqlx::query!(
    r#"
SELECT count(1) as size FROM fullfillment WHERE requirement = ?1
    "#,
    requirement
  )
  .fetch_one(&pool)
  .await?;

  if existing.size as i64 >= maximum.size {
    return Err(AppError::Server(format!("Maximum number of user for this requirement exeeded: {requirement}")))
  }

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

  Ok((StatusCode::CREATED, ()))
}

pub async fn delete(
  Path((user_id, requirement_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<()> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM fullfillment
  WHERE user = ?1 AND requirement = ?2
      "#,
      user_id, requirement_id
    )
    .execute(&pool)
    .await?;

  Ok((StatusCode::NO_CONTENT, ()))
}


#[derive(Deserialize)]
pub struct CreateFullfillment {
  requirement: i64,
  user: i64,
}
