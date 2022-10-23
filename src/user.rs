use axum::{
    Json, Extension, extract::Path,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self, AppError}, auth::{generate_salt, get_salted_password, generate_jwt, UserAuth, user_action_authorization}, db_modeling, utils::AppReponse};

#[derive(Deserialize)]
pub struct CreateUser {
  username: String,
  password: String,
}

#[derive(Deserialize)]
pub struct UpdateUser {
  username: Option<String>,
}

#[derive(Serialize)]
pub struct User {
  pub id: i64,
  pub username: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UserAuthReqData {
  username: String,
  password: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct UserAuthRespData {
  id: i64,
  token: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct UsedRequirements {
  name: String,
  score: i64,
}

pub async fn create(
  Json(payload): Json<CreateUser>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<Json<User>> {
  let salt = generate_salt();
  let password = get_salted_password(&payload.password, &salt.clone());

  let id = sqlx::query!(
      r#"
  INSERT INTO user ( username, password, salt )
  VALUES ( ?1, ?2, ?3 )
      "#,
      payload.username, password, salt
    )
    .execute(&pool)
    .await?
    .last_insert_rowid();

  let user = User {
    id,
    username: payload.username,
  };

  Ok((StatusCode::CREATED, Json(user)))
}

pub async fn single(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
  UserAuth(_auth_userid): UserAuth,
) -> AppReponse<Json<User>> {
  let user = sqlx::query_as!(User,
      r#"
  SELECT id, username FROM user
  WHERE ID = ?1
      "#,
      id
    )
    .fetch_optional(&pool)
    .await?;

  match user {
    Some(u) => Ok((StatusCode::OK, Json(u))),
    None => Err(AppError::NotFound(format!("{id}"))),
  }
}

pub async fn used_requirements(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<Json<Vec<UsedRequirements>>> {
  user_action_authorization(id, auth_userid, "cannot get requirements of another user")?;

  let requirements = sqlx::query_as!(UsedRequirements,
      r#"
  SELECT COUNT(requirement.name) AS score, requirement.name
  FROM requirement
  JOIN event ON requirement.event = event.id
  WHERE event.creator = ?1
  GROUP BY requirement.name
  LIMIT 10
      "#,
      id
    )
    .fetch_all(&pool)
    .await?;

  Ok((StatusCode::OK, Json(requirements)))
}

// pub async fn all(
//   Extension(pool): Extension<DbState>,
//   UserAuth(auth_userid): UserAuth,
// ) -> AppReponse<Json<Vec<User>>> {
//   let users = sqlx::query_as!(User, "SELECT id, username FROM user")
//     .fetch_all(&pool)
//     .await?;

//   Ok((StatusCode::OK, Json(users)))
// }

pub async fn update(
  Path(id): Path<i64>,
  Json(payload): Json<UpdateUser>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  user_action_authorization(id, auth_userid, "cannot update another user")?;

  if let Some(username) = payload.username {
    let _ = sqlx::query!(
      r#"
  UPDATE user SET username = ?1
  WHERE ID = ?2
      "#,
      username, id
    )
    .execute(&pool)
    .await?;

    Ok(((StatusCode::NO_CONTENT), ()))
  } else {
    Err(AppError::BadRequest(String::from("at least one field must be filled out")))
  }
}

pub async fn delete(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
  UserAuth(auth_userid): UserAuth,
) -> AppReponse<()> {
  user_action_authorization(id, auth_userid, "cannot delete another user")?;

  db_modeling::delete_db_user(&pool, id)
    .await
    .and_then(|r| Ok(((StatusCode::NO_CONTENT), r)))
}

pub async fn authentificate(
  Json(data): Json<UserAuthReqData>,
  Extension(pool): Extension<DbState>,
) -> AppReponse<Json<UserAuthRespData>> {
  let user_db = sqlx::query!(
      "
      SELECT id, password, salt
      FROM user
      WHERE username = ?
      ",
      data.username
    )
    .fetch_one(&pool)
    .await?;

  if user_db.id.is_none() {
    return Err(AppError::NotFound(format!("User doesn't exist")))
  }
  let user_id = user_db.id.unwrap();
  let calculated_password = get_salted_password(&data.password, &user_db.salt);
  if calculated_password != user_db.password {
    return Err(error::AppError::Unauthorized(String::from("incorrect password")));
  }

  let resp = UserAuthRespData {
    id: user_id,
    token: generate_jwt(&format!("{}", user_id), &data.username)
  };
  Ok((StatusCode::OK, Json(resp)))
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
    async fn register() {
      let (app, _) = setup_with_structure().await;
      let body_json = json!({
        "username": "Janko Hrasko",
        "password": "5e884898da28047151d0e56f8dc6292773603d0d6aabbdd62a11ef721d1542d8"
      });
      let expected_response = json!({
        "id": 1,
        "username": "Janko Hrasko"
      });

      let response = test_api(app, "/register", http::Method::POST, Some(body_json), StatusCode::CREATED, None).await;
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
        "username": "username1"
      });

      let response = test_api(app, "/user/1", http::Method::GET, None, StatusCode::OK, Some(("1", "username1"))).await;
      assert_eq!(response, Some(expected_response));
    }

    mod used_requirements {
      use super::*;

      #[tokio::test]
      async fn none() {
        let (app, _) = setup_with_data().await;
        let expected_response = json!([]);

        let response = test_api(app, "/user/3/requirements", http::Method::GET, None, StatusCode::OK, Some(("3", "username3"))).await;
        assert_eq!(response, Some(expected_response));
      }

      #[tokio::test]
      async fn over_limit() {
        let (app, pool) = setup_with_data().await;

        for i in 0..15 {
          let sql = format!(r#"INSERT INTO requirement (name, description, event, size) VALUES ("req-insert-{i}", "req1-desc", ?1, 2)"#);
          let _ = sqlx::QueryBuilder::new(sql)
            .build()
            .bind(1)
            .execute(&pool)
            .await
            .unwrap();
        }

        let response = test_api(app, "/user/1/requirements", http::Method::GET, None, StatusCode::OK, Some(("1", "username1"))).await;
        assert_eq!(response.unwrap().as_array().unwrap().len(), 10);
      }

      #[tokio::test]
      async fn standard() {
        let (app, _) = setup_with_data().await;
        let expected_response = json!([
          {
            "name": "req1",
            "score": 2,
          },
          {
            "name": "req2",
            "score": 1,
          }
        ]);

        let response = test_api(app, "/user/1/requirements", http::Method::GET, None, StatusCode::OK, Some(("1", "username1"))).await;
        assert_eq!(response, Some(expected_response));
      }

      #[tokio::test]
      async fn for_another() {
        let (app, _) = setup_with_data().await;
        let _ = test_api(app, "/user/1/requirements", http::Method::GET, None, StatusCode::FORBIDDEN, Some(("2", "username2"))).await;
      }
    }
  }

  mod update {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "username": "edited_username",
      });
      let _ = test_api(app, "/user/1", http::Method::PUT, Some(body_json), StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let result = sqlx::query!("select * from user where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.username, "edited_username");
    }

    #[tokio::test]
    async fn for_another() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "username": "edited_username",
      });
      let _ = test_api(app, "/user/2", http::Method::PUT, Some(body_json), StatusCode::FORBIDDEN, Some(("1", "username1"))).await;
    }
  }

  mod delete {
    use super::*;

    #[tokio::test]
    async fn simple() {
      let (app, pool) = setup_with_data().await;
      let _ = test_api(app, "/user/1", http::Method::DELETE, None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let results = sqlx::query!("select * from user")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 5);
      assert_eq!(results[0].id, 2);
      assert_eq!(results[1].id, 3);
      assert_eq!(results[2].id, 4);
      assert_eq!(results[3].id, 5);
      assert_eq!(results[4].id, 6);
    }

    #[tokio::test]
    async fn with_fk() {
      let (app, pool) = setup_with_data().await;
      let _ = test_api(app, "/user/1", http::Method::DELETE, None, StatusCode::NO_CONTENT, Some(("1", "username1"))).await;

      let results = sqlx::query!("select * from user")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 5);
      assert_eq!(results[0].id, 2);
      assert_eq!(results[1].id, 3);
      assert_eq!(results[2].id, 4);
      assert_eq!(results[3].id, 5);
      assert_eq!(results[4].id, 6);

      //TODO cleanup checks
    }

    #[tokio::test]
    async fn for_another() {
      let (app, _) = setup_with_data().await;
      let _ = test_api(app, "/user/2", http::Method::DELETE, None, StatusCode::FORBIDDEN, Some(("1", "username1"))).await;
    }
  }
}