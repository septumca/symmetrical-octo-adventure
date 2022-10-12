use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self, AppError}, db_modeling::{Updatable, self}};

pub async fn create(
  Json(payload): Json<CreateRequirement>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<Requirement>, error::AppError> {
  let CreateRequirement { name, description, event } = payload;
  let id = sqlx::query!(
      r#"
  INSERT INTO requirement ( name, description, event )
  VALUES ( ?1, ?2, ?3 )
      "#,
      name, description, event
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let event = Requirement {
    id,
    name,
    description,
    event,
  };

  Ok(Json(event))
}

pub async fn update(
  Path(id): Path<i64>,
  Json(payload): Json<UpdateRequirement>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  if !payload.validate() {
    return Err(AppError::BadRequest(String::from("at least one field must be filled out")));
  }
  let sql = format!("UPDATE requirement SET {} WHERE id = ?1", payload.update_string());
  let _ = sqlx::QueryBuilder::new(sql)
    .build()
    .bind(id)
    .execute(&pool)
    .await?;

  Ok(())
}

pub async fn delete(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  db_modeling::delete_db_requirement(&pool, id).await
}


#[derive(Deserialize)]
pub struct CreateRequirement {
  name : String,
  description: Option<String>,
  event: i64,
}

#[derive(Deserialize)]
pub struct UpdateRequirement {
  name: Option<String>,
  description: Option<String>,
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
    updates.join(", ")
  }

  fn validate(&self) -> bool {
    self.name.is_some() || self.description.is_some()
  }
}

#[derive(Serialize)]
pub struct Requirement {
  id: i64,
  name: String,
  description: Option<String>,
  event: i64
}
