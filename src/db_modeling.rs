use axum::Extension;

use crate::{DbState, error};

pub async fn database_up(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  sqlx::query_file!("./sql/up.sql").execute(&pool).await.unwrap();

  Ok(())
}

pub async fn database_down(
  Extension(pool): Extension<DbState>
) -> Result<(), error::AppError> {
  sqlx::query_file!("./sql/down.sql").execute(&pool).await.unwrap();
  Ok(())
}

pub trait Updatable {
  fn validate(&self) -> bool;
  fn update_string(&self) -> String;
}


pub async fn delete_db_user(pool: &DbState, id: i64) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
    r#"
DELETE FROM fullfillment
WHERE requirement in
  (SELECT id FROM requirement WHERE requirement.event in
    (SELECT id FROM event WHERE event.creator = ?1)
  )
OR user = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
    r#"
DELETE FROM requirement
WHERE event in (SELECT id FROM event WHERE event.creator = ?1)
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
    r#"
DELETE FROM participant
WHERE event in (SELECT id FROM event WHERE event.creator = ?1)
OR user = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
      r#"
  DELETE FROM event
  WHERE ID in (SELECT id FROM event WHERE event.creator = ?1)
      "#,
      id
    )
    .execute(pool)
    .await?;


  let _ = sqlx::query!(
      r#"
  DELETE FROM user
  WHERE ID = ?1
      "#,
      id
    )
    .execute(pool)
    .await?;

  Ok(())
}

pub async fn delete_db_event(pool: &DbState, id: i64) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
    r#"
DELETE FROM fullfillment
WHERE requirement in (SELECT id FROM requirement WHERE requirement.event = ?1)
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
    r#"
DELETE FROM requirement
WHERE event = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
    r#"
DELETE FROM participant
WHERE event = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
      r#"
  DELETE FROM event
  WHERE ID = ?1
      "#,
      id
    )
    .execute(pool)
    .await?;

  Ok(())
}

pub async fn delete_db_requirement(pool: &DbState, id: i64) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
    r#"
DELETE FROM fullfillment
WHERE requirement = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  let _ = sqlx::query!(
    r#"
DELETE FROM requirement
WHERE id = ?1
    "#,
    id
  )
  .execute(pool)
  .await?;

  Ok(())
}
