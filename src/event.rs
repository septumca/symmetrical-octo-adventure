use axum::{
  Json, Extension, extract::{Path, Query},
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{AppError}, db_modeling::{Updatable, self}, user::User, utils::AppReponse, auth::{UserAuth, event_action_authorization, user_action_authorization}};

#[derive(Serialize)]
pub struct UpdateEventResponse {
  name: Option<String>,
  time: Option<i64>,
  description: Option<String>,
}

#[derive(sqlx::FromRow, Serialize)]
pub struct DbEvent {
  id: i64,
  name: String,
  description: Option<String>,
  time: i64,
  creator: i64,
  username: String,
}

#[derive(Serialize)]
pub struct Event {
  id: i64,
  name: String,
  description: Option<String>,
  time: i64,
  creator: User,
}


#[derive(Serialize)]
pub struct EventDetail {
  id: i64,
  name: String,
  description: Option<String>,
  time: i64,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateRequirement {
  id: Option<i64>,
  name: String,
  description: Option<String>,
  size: Option<i64>,
}

pub async fn create(
  Json(payload): Json<CreateEvent>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<Event>> {
  let CreateEvent { name, description, time, creator } = payload;
  user_action_authorization(creator, auth_userid, "cannot create event as another user")?;

  let id = sqlx::query!(
      r#"
  INSERT INTO event ( name, description, time,creator )
  VALUES ( ?1, ?2, ?3, ?4 )
      "#,
      name, description, time, creator
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
    },
    time,
  };

  Ok((StatusCode::CREATED, Json(event)))
}

pub async fn single(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<Json<EventDetail>> {
  let event = sqlx::query_as!(DbEvent,
      r#"
  SELECT event.id, name, description, time, creator, user.username
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
      time: d.time,
      creator: User {
        id: d.creator,
        username: d.username,
      },
      participants,
      requirements,
      fullfillments,
    };
    Ok((StatusCode::OK, Json(event_detail)))
  } else {
    Err(AppError::NotFound(format!("{id}")))
  }
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct EventSearchParam {
    page: Option<u32>,
    pageSize: Option<u32>,
}

pub async fn all(
  Extension(pool): Extension<DbState>,
  Query(params): Query<EventSearchParam>
) -> AppReponse<Json<Vec<Event>>> {
  let mut sql = "SELECT event.id, name, description, creator, time, user.username
  FROM event
  JOIN user ON event.creator = user.id ".to_owned();
  if let (Some(page), Some(page_size)) = (params.page, params.pageSize) {
    sql += &format!("LIMIT {} OFFSET {}", page_size, (page-1) * page_size);
  }

  let events = sqlx::QueryBuilder::new(sql)
    .build_query_as()
    .fetch_all(&pool)
    .await?
    .into_iter()
    .map(|dbevent: DbEvent| Event {
      id: dbevent.id,
      name: dbevent.name,
      description: dbevent.description,
      time: dbevent.time,
      creator: User {
        id: dbevent.creator,
        username: dbevent.username,
      }
    })
    .collect();

  Ok((StatusCode::OK, Json(events)))
}

pub async fn update(
  Path(id): Path<i64>,
  Json(payload): Json<UpdateEvent>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<UpdateEventResponse>> {
  if !payload.validate() {
    return Err(AppError::BadRequest(String::from("at least one field must be filled out")));
  }
  event_action_authorization(&pool, id, auth_userid, "cannot change event that user doesn't own").await?;

  let sql = format!("UPDATE event SET {} WHERE id = ?1", payload.update_string());
  let _ = sqlx::QueryBuilder::new(sql)
    .build()
    .bind(id)
    .execute(&pool)
    .await?;

  let response = UpdateEventResponse {
    name: payload.name,
    time: payload.time,
    description: payload.description,
  };

  Ok((StatusCode::OK, Json(response)))
}

pub async fn delete(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  event_action_authorization(&pool, id, auth_userid, "cannot delete event that user doesn't own").await?;

  db_modeling::delete_db_event(&pool, id)
    .await
    .and_then(|r| Ok((StatusCode::NO_CONTENT, r)))
}


#[derive(Deserialize)]
pub struct CreateEvent {
  name : String,
  description: Option<String>,
  time: i64,
  creator: i64,
}

#[derive(Deserialize)]
pub struct UpdateEvent {
  name: Option<String>,
  description: Option<String>,
  time: Option<i64>,
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
    if let Some(time) = &self.time {
      updates.push(format!("time = {time}"));
    }
    updates.join(", ")
  }

  fn validate(&self) -> bool {
    self.name.is_some() || self.description.is_some() || self.time.is_some()
  }
}

#[cfg(test)]
mod test {
  use super::*;
  use serde_json::json;
  use crate::utils::test::{setup_with_structure, test_api, setup_with_data};
  use axum::http;

  mod create {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_structure().await;
      let body_json = json!({
        "name": "my new event",
        "description": "my event description",
        "time": 1664928000,
        "creator": 1
      });
      let expected_response = json!({
        "id": 1,
        "name": "my new event",
        "description": "my event description",
        "time": 1664928000,
        "creator": {
          "id": 1,
          "username": "username1"
        }
      });
      let _ = sqlx::query!("INSERT INTO user (id, username, password, salt) VALUES (1, 'username1', 'sha256password', 'somesalt')")
        .execute(&pool)
        .await
        .unwrap();

        let response = test_api(app, "/event", http::Method::POST, Some(body_json), StatusCode::CREATED, Some(("1", "username1"))).await;
        assert_eq!(response, Some(expected_response));
    }
  }

  mod get {
    use super::*;

    #[tokio::test]
    async fn single() {
      let (app, _) = setup_with_data().await;
      let expected_response = json!({
        "id": 1,
        "name": "event-1",
        "description": "some description 1",
        "participants": [
          { "id": 2, "username": "username2" },
          { "id": 3, "username": "username3" }
        ],
        "requirements": [
          { "id": 1, "name": "req1", "description": "req1-desc", "size": 2 },
          { "id": 2, "name": "req2", "description": "req2-desc", "size": 1 }
        ],
        "fullfillments": [{
          "requirement": 1,
          "user": {
            "id": 4,
            "username": "username4"
          }
        }],
        "creator": {
          "id": 1,
          "username": "username1"
        },
        "time": 1664928000
      });

      let response = test_api(app, "/event/1", http::Method::GET, None, StatusCode::OK, None).await;
      assert_eq!(response, Some(expected_response));
    }

    #[tokio::test]
    async fn all() {
      let (app, _) = setup_with_data().await;
      let expected_response = json!([
        {
          "id": 1,
          "name": "event-1",
          "description": "some description 1",
          "creator": {
            "id": 1,
            "username": "username1"
          },
          "time": 1664928000
        },
        {
          "id": 2,
          "name": "event-2",
          "description": "some description 2",
          "creator": {
            "id": 6,
            "username": "username6"
          },
          "time": 1664928000
        },
        {
          "id": 3,
          "name": "event-3",
          "description": "some description 3",
          "creator": {
            "id": 4,
            "username": "username4"
          },
          "time": 1664928000
        },
        {
          "id": 4,
          "name": "event-4",
          "description": "some description 4",
          "creator": {
            "id": 1,
            "username": "username1"
          },
          "time": 1664928000
        }
      ]);

      let response = test_api(app, "/event", http::Method::GET, None, StatusCode::OK, None).await;
      assert_eq!(response, Some(expected_response));
    }

    #[tokio::test]
    async fn all_paginated() {
      let (app, _) = setup_with_data().await;
      let expected_response = json!([
        {
          "id": 2,
          "name": "event-2",
          "description": "some description 2",
          "creator": {
            "id": 6,
            "username": "username6"
          },
          "time": 1664928000
        }
      ]);

      let response = test_api(app, "/event?page=2&pageSize=1", http::Method::GET, None, StatusCode::OK, None).await;
      assert_eq!(response, Some(expected_response));
    }
  }

  mod update {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "name": "edited name event 1",
        "description": "some description 1",
      });

      let _ = test_api(app, "/event/1", http::Method::PUT, Some(body_json), StatusCode::OK, Some(("1", "username1"))).await;

      let result = sqlx::query!("select * from event where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.name, "edited name event 1");
      assert_eq!(result.description, Some("some description 1".to_owned()));
    }

    #[tokio::test]
    async fn time() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "time": 1633392000,
      });

      let _ = test_api(app, "/event/1", http::Method::PUT, Some(body_json), StatusCode::OK, Some(("1", "username1"))).await;

      let result = sqlx::query!("select * from event where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.time, 1633392000);
    }
  }

  mod delete {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;

      let events = sqlx::query!("SELECT COUNT(id) as cnt FROM event")
        .fetch_one(&pool)
        .await
        .unwrap();

      let _ = test_api(app, "/event/3", http::Method::DELETE, None, StatusCode::NO_CONTENT, Some(("4", "username4"))).await;

      let results = sqlx::query!("select * from event")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), events.cnt as usize - 1);
      assert!(results.iter().all(|r| r.id != 3 && r.name != "event-3"));
    }

    #[tokio::test]
    async fn with_fk() {
      let (app, pool) = setup_with_data().await;

      let events = sqlx::query!("SELECT COUNT(id) as cnt FROM event")
        .fetch_one(&pool)
        .await
        .unwrap();

      let _ = test_api(app, "/event/1", http::Method::DELETE, None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let results = sqlx::query!("select * from event")
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(results.len(), events.cnt as usize - 1);
        assert!(results.iter().all(|r| r.id != 1 && r.name != "event-1"));

      //TODO cleanup checks
    }
  }

}