use axum::{
  Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, utils::AppReponse, error::AppError, auth::UserAuth};

#[derive(Deserialize)]
pub struct CreateParticipant {
  event: i64,
  user: i64,
}


#[derive(Serialize)]
pub struct CreateParticipantResponse {
  username: String,
  user: i64,
}

pub async fn create(
  Json(payload): Json<CreateParticipant>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<CreateParticipantResponse>> {
  let CreateParticipant { event, user } = payload;
  if user != auth_userid {
    return Err(AppError::Unauthorized(format!("cannot make participation for another user")));
  }

  let selected_user = sqlx::query!(
      r#"
  SELECT username FROM user WHERE id = ?1
      "#,
      user
    )
    .fetch_one(&pool)
    .await?;

  let _ = sqlx::query!(
      r#"
  INSERT INTO participant ( event, user )
  VALUES ( ?1, ?2 )
      "#,
      event, user
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let participant = CreateParticipantResponse {
    user,
    username: selected_user.username
  };
  Ok((StatusCode::CREATED, Json(participant)))
}

pub async fn delete(
  Path((user_id, event_id)): Path<(i64, i64)>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  if user_id != auth_userid {
    return Err(AppError::Unauthorized(format!("cannot remove  participation for another user")));
  }
  let _ = sqlx::query!(
      r#"
  DELETE FROM participant
  WHERE user = ?1 AND event = ?2
      "#,
      user_id, event_id
    )
    .execute(&pool)
    .await?;

  Ok((StatusCode::NO_CONTENT, ()))
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
    async fn without_auth() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "event": 3,
        "user": 1,
      });

      test_api(app, "/participant", http::Method::POST, Some(body_json), None, StatusCode::UNAUTHORIZED, None).await;
    }

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "event": 3,
        "user": 1,
      });
      let expected_response = json!({
        "user": 1,
        "username": "username1"
      });

      test_api(app, "/participant", http::Method::POST, Some(body_json), Some(expected_response), StatusCode::CREATED, Some(("1", "username1"))).await;

      let results = sqlx::query!("select * from participant where event = 3 and user = 1")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn for_anothe() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "event": 3,
        "user": 6,
      });

      test_api(app, "/participant", http::Method::POST, Some(body_json), None, StatusCode::UNAUTHORIZED, Some(("1", "username1"))).await;
    }
  }

  mod delete {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;

      test_api(app, "/participant/3/1", http::Method::DELETE, None, None, StatusCode::NO_CONTENT, Some(("3", "username3"))).await;

      let results = sqlx::query!("select user, event from participant")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 3);
      println!("{:?}", results);
      assert_eq!(results[0].user, 2);
      assert_eq!(results[0].event, 1);
      assert_eq!(results[1].user, 3);
      assert_eq!(results[1].event, 2);
      assert_eq!(results[2].user, 4);
      assert_eq!(results[2].event, 2);
    }
  }
}