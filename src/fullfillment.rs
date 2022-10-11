use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self}};

pub async fn create(
  Json(payload): Json<CreateFullfillment>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<Fullfillment>, error::AppError> {
  let CreateFullfillment { requirement, user } = payload;
  let id = sqlx::query!(
      r#"
  INSERT INTO fullfillment ( requirement, user )
  VALUES ( ?1, ?2 )
      "#,
      requirement, user
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let fullfillment = Fullfillment {
    id,
    requirement,
    user
  };

  Ok(Json(fullfillment))
}

pub async fn delete(
  Path(id): Path<String>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM fullfillment
  WHERE ID = ?1
      "#,
      id
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

#[derive(Serialize)]
pub struct Fullfillment {
  pub id: i64,
  pub requirement: i64,
  pub user: i64,
}