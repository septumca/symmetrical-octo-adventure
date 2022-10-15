use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{AppError}, db_modeling::{Updatable, self}, utils::AppReponse};

pub async fn create(
  Json(payload): Json<CreateRequirement>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<Json<Requirement>> {
  let CreateRequirement { name, description, event, size } = payload;
  let size = size.unwrap_or(1);
  let id = sqlx::query!(
      r#"
  INSERT INTO requirement ( name, description, event, size )
  VALUES ( ?1, ?2, ?3, ?4 )
      "#,
      name, description, event, size
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let event = Requirement {
    id,
    name,
    description,
    event,
    size,
  };

  Ok((StatusCode::CREATED, Json(event)))
}

pub async fn update(
  Path(id): Path<i64>,
  Json(payload): Json<UpdateRequirement>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<()> {
  if !payload.validate() {
    return Err(AppError::BadRequest(String::from("at least one field must be filled out")));
  }
  let sql = format!("UPDATE requirement SET {} WHERE id = ?1", payload.update_string());
  let _ = sqlx::QueryBuilder::new(sql)
    .build()
    .bind(id)
    .execute(&pool)
    .await?;

  Ok((StatusCode::NO_CONTENT, ()))
}

pub async fn delete(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<()> {
  db_modeling::delete_db_requirement(&pool, id)
    .await
    .and_then(|r| Ok((StatusCode::NO_CONTENT, r)))
}


#[derive(Deserialize)]
pub struct CreateRequirement {
  name : String,
  description: Option<String>,
  size: Option<i64>,
  event: i64,
}

#[derive(Deserialize)]
pub struct UpdateRequirement {
  name: Option<String>,
  description: Option<String>,
  size: Option<i64>,
}

impl Updatable for UpdateRequirement {
  fn update_string(&self) -> String {
    let mut updates = vec![];
    if let Some(name) = &self.name {
      updates.push(format!("name = '{name}'"));
    }
    if let Some(description) = &self.description {
      updates.push(format!("description = '{description}'"));
    }
    if let Some(size) = &self.size {
      updates.push(format!("description = {size}"));
    }
    updates.join(", ")
  }

  fn validate(&self) -> bool {
    self.name.is_some() || self.description.is_some() || self.size.is_some()
  }
}

#[derive(Serialize)]
pub struct Requirement {
  id: i64,
  name: String,
  description: Option<String>,
  size: i64,
  event: i64
}
