use axum::{
  Json, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self, AppError}, db_modeling::{Updatable, self}, user::User};

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

    let user = sqlx::query_as!(User,
      r#"
  SELECT id, username
  FROM user
  WHERE id = ?1
      "#,
      creator
    )
    .fetch_one(&pool)
    .await?;

  let event = Event {
    id,
    name,
    description,
    creator: User {
      id: creator,
      username: user.username
    }
  };

  Ok(Json(event))
}

pub async fn single(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<EventDetail>, error::AppError> {
  let event = sqlx::query_as!(DbEvent,
      r#"
  SELECT event.id, name, description, creator, user.username
  FROM event
  JOIN user ON event.creator = user.id
  WHERE event.id = ?1
      "#,
      id
    )
    .fetch_optional(&pool)
    .await?;

  if let Some(d) = event {
    let participants = sqlx::query_as!(User,
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
  SELECT id, name, description, size FROM requirement
  WHERE requirement.event = ?1
      "#,
      id
    )
    .fetch_all(&pool)
    .await?;

    let fullfillments = sqlx::query!(
      r#"
  SELECT user.id,  user.username, requirement
  FROM fullfillment
  JOIN user on fullfillment.user = user.id
  WHERE fullfillment.requirement in (
      select id from requirement
      where requirement.event = ?1
    )
      "#,
      id
    )
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|f| Fullfillment {
      user: User {
        id: f.id,
        username: f.username
      },
      requirement: f.requirement
    })
    .collect();

    let event_detail = EventDetail {
      id,
      name: d.name,
      description: d.description,
      creator: User {
        id: d.creator,
        username: d.username,
      },
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
  let events = sqlx::query_as!(DbEvent, "
  SELECT event.id, name, description, creator, user.username
  FROM event
  JOIN user ON event.creator = user.id")
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|dbevent| Event {
      id: dbevent.id,
      name: dbevent.name,
      description: dbevent.description,
      creator: User {
        id: dbevent.creator,
        username: dbevent.username,
      }
    })
    .collect();

  Ok(Json(events))
}

pub async fn update(
  Path(id): Path<i64>,
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
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  db_modeling::delete_db_event(&pool, id).await
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
pub struct DbEvent {
  id: i64,
  name: String,
  description: Option<String>,
  creator: i64,
  username: String,
}

#[derive(Serialize)]
pub struct Event {
  id: i64,
  name: String,
  description: Option<String>,
  creator: User,
}


#[derive(Serialize)]
pub struct EventDetail {
  id: i64,
  name: String,
  description: Option<String>,
  participants: Vec<User>,
  requirements: Vec<Requirement>,
  fullfillments: Vec<Fullfillment>,
  creator: User,
}

#[derive(Serialize)]
struct Fullfillment {
  requirement: i64,
  user: User,
}

#[derive(Serialize)]
struct Requirement {
  id: i64,
  name: String,
  description: Option<String>,
  size: i64,
}