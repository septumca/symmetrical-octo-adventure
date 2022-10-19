use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{AppError}, db_modeling::{Updatable, self}, utils::AppReponse, auth::{UserAuth, event_action_authorization, requirement_action_authorization}};

#[derive(Serialize)]
pub struct Requirement {
  id: i64,
  name: String,
  description: Option<String>,
  size: i64,
  event: i64
}

pub async fn create(
  Json(payload): Json<CreateRequirement>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<Requirement>> {
  let CreateRequirement { name, description, event, size } = payload;
  event_action_authorization(&pool, event, auth_userid, "cannot create requirement for event that user doesn't own").await?;

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
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  if !payload.validate() {
    return Err(AppError::BadRequest(String::from("at least one field must be filled out")));
  }
  requirement_action_authorization(&pool, id, auth_userid, "cannot create requirement for event that user doesn't own").await?;

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
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  requirement_action_authorization(&pool, id, auth_userid, "cannot delete requirement for event that user doesn't own").await?;

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

#[cfg(test)]
mod test {
  use super::*;
  use serde_json::json;
  use crate::utils::test::{test_api, setup_with_data};
  use axum::http;

  mod create {
    use super::*;

    #[tokio::test]
    async fn default_size() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "name": "new-req",
        "description": "new-req-desc",
        "event": 3
      });
      let expected_response = json!({
        "id": 4,
        "name": "new-req",
        "description": "new-req-desc",
        "event": 3,
        "size": 1
      });

      test_api(app, "/requirement", http::Method::POST, Some(body_json), Some(expected_response), StatusCode::CREATED, Some(("4", "username4"))).await;
    }

    #[tokio::test]
    async fn custom_size() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "name": "new-req",
        "description": "new-req-desc",
        "event": 3,
        "size": 5
      });
      let expected_response = json!({
        "id": 4,
        "name": "new-req",
        "description": "new-req-desc",
        "event": 3,
        "size": 5
      });

      test_api(app, "/requirement", http::Method::POST, Some(body_json), Some(expected_response), StatusCode::CREATED, Some(("4", "username4"))).await;
    }
  }

  mod update {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "name": "edited req name",
        "description": "some other description 1",
      });

      test_api(app, "/requirement/1", http::Method::PUT, Some(body_json), None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let result = sqlx::query!("select * from requirement where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.name, "edited req name");
      assert_eq!(result.description, Some("some other description 1".to_owned()));
    }

    #[tokio::test]
    async fn for_another() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "name": "edited req name",
        "description": "some other description 1",
      });

      test_api(app, "/requirement/1", http::Method::PUT, Some(body_json), None, StatusCode::UNAUTHORIZED, Some(("2", "username2"))).await;
    }
  }

  mod delete {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;

      test_api(app, "/requirement/2", http::Method::DELETE, None, None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let results = sqlx::query!("select id from requirement")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 2);
      assert_eq!(results[0].id, 1);
      assert_eq!(results[1].id, 3);
    }

    #[tokio::test]
    async fn with_fk() {
      let (app, pool) = setup_with_data().await;

      test_api(app, "/requirement/1", http::Method::DELETE, None, None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let results = sqlx::query!("select id from requirement")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 2);
      assert_eq!(results.len(), 2);
      assert_eq!(results[0].id, 2);
      assert_eq!(results[1].id, 3);

      //TODO cleanup checks
    }

    #[tokio::test]
    async fn for_another() {
      let (app, _) = setup_with_data().await;
      test_api(app, "/requirement/1", http::Method::DELETE, None, None, StatusCode::UNAUTHORIZED, Some(("2", "username2"))).await;
    }
  }
}