use axum::Extension;

use crate::{DbState, error, utils::read_file};

pub async fn database_up(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  let sql = read_file("up.sql")?;
  sqlx::query(&sql).execute(&pool).await?;

  Ok(())
}

pub async fn database_down(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  let sql = read_file("down.sql")?;
  sqlx::query(&sql).execute(&pool).await?;
  Ok(())
}

pub trait Updatable {
  fn validate(&self) -> bool;
  fn update_string(&self) -> String;
}