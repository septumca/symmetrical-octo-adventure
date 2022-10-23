use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{AppError}, utils::AppReponse, auth::{UserAuth, user_action_authorization}, user::User};


#[derive(Deserialize)]
pub struct CreateFullfillment {
  requirement: i64,
  user: i64,
}

#[derive(Serialize)]
pub struct CreateFullfillmentResponse {
  requirement: i64,
  user: User,
}

pub async fn create(
  Json(payload): Json<CreateFullfillment>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<CreateFullfillmentResponse>> {
  let CreateFullfillment { requirement, user } = payload;
  user_action_authorization(user, auth_userid, "cannot add fullfillment for another user")?;
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

    let dbuser = sqlx::query!(
      r#"
  SELECT username FROM user WHERE id = ?1
      "#,
      user
    )
    .fetch_one(&pool)
    .await?;

  let response = CreateFullfillmentResponse {
    requirement,
    user: User {
      id: user,
      username: dbuser.username
    }
  };

  Ok((StatusCode::CREATED, Json(response)))
}

pub async fn delete(
  Path((user_id, requirement_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  user_action_authorization(user_id, auth_userid, "cannot remove fullfillment for another user")?;

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


#[cfg(test)]
mod fullfillment {
  use super::*;
  use serde_json::json;
  use crate::utils::test::{test_api, setup_with_data};
  use axum::http;

  mod create {
    use super::*;

    #[tokio::test]
    async fn to_single() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 2,
        "user": 6,
      });
      let expected_response = json!({
        "requirement": 2,
        "user": {
          "id": 6,
          "username": "username6"
        }
      });

      let response = test_api(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::CREATED, Some(("6", "username6"))).await;
      assert_eq!(response, Some(expected_response));
    }

    #[tokio::test]
    async fn to_mutliple() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 1,
        "user": 6,
      });
      let expected_response = json!({
        "requirement": 1,
        "user": {
          "id": 6,
          "username": "username6"
        }
      });

      let response = test_api(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::CREATED, Some(("6", "username6"))).await;
      assert_eq!(response, Some(expected_response));
    }

    #[tokio::test]
    async fn to_non_existing() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 5,
        "user": 6,
      });

      let _ = test_api(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::NOT_FOUND, Some(("6", "username6"))).await;
    }

    #[tokio::test]
    async fn fully_assigned() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 3,
        "user": 6,
      });

      let _ = test_api(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::INTERNAL_SERVER_ERROR, Some(("6", "username6"))).await;
    }
  }

  mod delete {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;

      let _ = test_api(app, "/fullfillment/4/1", http::Method::DELETE, None, StatusCode::NO_CONTENT, Some(("4", "username4"))).await;

      let results = sqlx::query!("select user, requirement from fullfillment")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 1);
      assert_eq!(results[0].user, 2);
      assert_eq!(results[0].requirement, 3);
    }
  }
}