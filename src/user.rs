use axum::{
    Json, Extension, extract::Path,
};
use serde::{Deserialize, Serialize};

use crate::{DbState, error::{self, AppError}, auth::{generate_salt, get_salted_password, generate_jwt}, db_modeling};

pub async fn create(
  Json(payload): Json<CreateUser>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<User>, error::AppError> {
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

  Ok(Json(user))
}

pub async fn single(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<User>, error::AppError> {
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
    Some(u) => Ok(Json(u)),
    None => Err(AppError::NotFound(format!("{id}"))),
  }
}

pub async fn all(
  Extension(pool): Extension<DbState>,
) -> Result<Json<Vec<User>>, error::AppError> {
  let users = sqlx::query_as!(User, "SELECT id, username FROM user")
    .fetch_all(&pool)
    .await?;

  Ok(Json(users))
}

pub async fn update(
  Path(id): Path<i64>,
  Json(payload): Json<UpdateUser>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
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

    Ok(())
  } else {
    Err(AppError::BadRequest(String::from("at least one field must be filled out")))
  }
}

pub async fn delete(
  Path(id): Path<i64>,
  Extension(pool): Extension<DbState>,
) -> Result<(), error::AppError> {
  db_modeling::delete_db_user(&pool, id).await
}

pub async fn authentificate(
  Json(data): Json<UserAuthReqData>,
  Extension(pool): Extension<DbState>,
) -> Result<Json<UserAuthRespData>, error::AppError> {
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

  let calculated_password = get_salted_password(&data.password, &user_db.salt);
  if calculated_password != user_db.password {
    return Err(error::AppError::Unauthorized(String::from("incorrect password")));
  }

  let resp = UserAuthRespData {
    id: user_db.id,
    token: generate_jwt(format!("{}", user_db.id))
  };
  Ok::<Json<UserAuthRespData>, error::AppError>(Json(resp))

}

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