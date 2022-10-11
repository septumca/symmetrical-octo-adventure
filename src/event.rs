use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self, AppError}, db_modeling::Updatable, requirement::Requirement, fullfillment::Fullfillment};

pub async fn create(
  Json(payload): Json<CreateEvent>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<Event>, error::AppError> {
  let CreateEvent { name, description, creator } = payload;
  let id = sqlx::query!(
      r#"
  INSERT INTO event ( name, description, creator )
  VALUES ( ?1, ?2, ?3 )
      "#,
      name, description, creator
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let event = Event {
    id,
    name,
    description,
    creator
  };

  Ok(Json(event))
}

pub async fn single(
  Path(id): Path<String>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<EventDetail>, error::AppError> {
  let id = id.parse::<i64>().expect("cannot convert id to integer");
  let event = sqlx::query_as!(Event,
      r#"
  SELECT id, name, description, creator FROM event
  WHERE ID = ?1
      "#,
      id
    )
    .fetch_optional(&pool)
    .await?;

  if let Some(d) = event {
    let participants = sqlx::query_as!(Participant,
      r#"
  SELECT id, username FROM user
  JOIN participant on participant.user = user.id
  WHERE participant.event = ?1
      "#,
      id
    )
    .fetch_all(&pool)
    .await?;

    let requirements = sqlx::query_as!(Requirement,
      r#"
  SELECT id, name, description FROM requirement
  WHERE requirement.event = ?1
      "#,
      id
    )
    .fetch_all(&pool)
    .await?;

    let fullfillments = sqlx::query_as!(Fullfillment,
      r#"
  SELECT id, requirement, user FROM fullfillment
  WHERE fullfillment.requirement in (
      select id from requirement
      where requirement.event = ?1
    )
      "#,
      id
    )
    .fetch_all(&pool)
    .await?;

    let event_detail = EventDetail {
      id,
      name: d.name,
      description: d.description,
      creator: d.creator,
      participants,
      requirements,
      fullfillments,
    };
    Ok(Json(event_detail))
  } else {
    Err(AppError::NotFound(format!("{id}")))
  }
}

  pub async fn all(
  Extension(pool): Extension<DbState>,
  ) -> Result<Json<Vec<Event>>, error::AppError> {
  let events = sqlx::query_as!(Event, "SELECT id, name, description, creator FROM event")
    .fetch_all(&pool)
    .await?;

  Ok(Json(events))
}

pub async fn update(
  Path(id): Path<String>,
  Json(payload): Json<UpdateEvent>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  if !payload.validate() {
    return Err(AppError::BadRequest(String::from("at least one field must be filled out")));
  }
  let sql = format!("UPDATE event SET {} WHERE id = ?1", payload.update_string());
  let _ = sqlx::QueryBuilder::new(sql)
    .build()
    .bind(id)
    .execute(&pool)
    .await?;

  Ok(())
}

pub async fn delete(
  Path(id): Path<String>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  let _ = sqlx::query!(
      r#"
  DELETE FROM event
  WHERE ID = ?1
      "#,
      id
    )
    .execute(&pool)
    .await?;

  Ok(())
}


#[derive(Deserialize)]
pub struct CreateEvent {
  name : String,
  description: Option<String>,
  creator: i64,
}

#[derive(Deserialize)]
pub struct UpdateEvent {
  name: Option<String>,
  description: Option<String>,
}

impl Updatable for UpdateEvent {
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
pub struct Participant {
  id: i64,
  username: String,
}

#[derive(Serialize)]
pub struct Event {
  id: i64,
  name: String,
  description: Option<String>,
  creator: i64,
}


#[derive(Serialize)]
pub struct EventDetail {
  id: i64,
  name: String,
  description: Option<String>,
  participants: Vec<Participant>,
  requirements: Vec<Requirement>,
  fullfillments: Vec<Fullfillment>,
  creator: i64,
}

