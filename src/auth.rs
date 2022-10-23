use axum::{async_trait, extract::{FromRequest, RequestParts}, Json};
use chrono::{Utc, Duration};
use hyper::StatusCode;
use jsonwebtoken::{EncodingKey, Header, encode, decode, DecodingKey, Validation};
use rand::Rng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::{env, collections::HashMap};
use reqwest;

use crate::{error::AppError, DbState, utils::AppReponse};

pub const ISSUER: &str = "zmtwc";

pub struct UserAuth(pub i64);

#[async_trait]
impl<B> FromRequest<B> for UserAuth
where
    B: Send,
{
    type Rejection = AppError;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
      let token = req.headers()
        .get("X-JWT-Token")
        .and_then(|header| header.to_str().ok());

      if let Some(token) = token {
        if let Some(claims) = validate_and_decode(token) {
          return Ok(UserAuth(claims.sub.parse::<i64>().unwrap()));
        } else {
          return Err(AppError::Unauthorized("Invalid JWT token".to_owned()))
        }
      }
      Err(AppError::Unauthorized("`X-JWT-Token` header is missing".to_owned()))
    }
}

#[derive(Deserialize)]
pub struct CaptchaRequest {
  token: String
}

#[derive(Debug, Deserialize)]
struct CaptchaResponse {
  success: bool,
  #[serde(rename(deserialize = "error-codes"))]
  error_codes: Option<Vec<String>>
}

pub async fn verify_captcha(
  Json(payload): Json<CaptchaRequest>,
) -> AppReponse<()> {
  let mut params = HashMap::new();
  params.insert("secret", env::var("CAPTCHA_SECRET_KEY").expect("captcha key must be set"));
  params.insert("response", payload.token);

  let client = reqwest::Client::new();

  let res = client.post("https://www.google.com/recaptcha/api/siteverify")
    .form(&params)
    .send()
    .await?;

  let text = res.text().await?;
  let response_body: CaptchaResponse = serde_json::from_str(&text)?;
  tracing::debug!("captcha verification response body: {:?}", response_body);

  if !response_body.success {
    return Err(AppError::BadRequest(format!("captcha verification failed: {}", response_body.error_codes.unwrap_or(vec!["Unknown error".to_owned()]).join(", "))));
  }

  Ok((StatusCode::OK, ()))
}



#[derive(Debug, Serialize, Deserialize)]
struct Claims {
  iss: String,
  sub: String,
  username: String,
  exp: i64,
  iat: i64,
}

impl Claims {
  pub fn new(sub: String, username: String) -> Self {
    let iat = Utc::now();
    let exp = iat + Duration::hours(24);

    Self {
      iss: String::from(ISSUER),
      sub,
      username,
      iat: iat.timestamp(),
      exp: exp.timestamp(),
    }
  }
}

pub fn generate_salt() -> String {
  const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                          abcdefghijklmnopqrstuvwxyz\
                          0123456789)(*&^%$#@!~";
  const SALT_LEN: usize = 8;
  let mut rng = rand::thread_rng();

  let salt: String = (0..SALT_LEN)
    .map(|_| {
      let idx = rng.gen_range(0..CHARSET.len());
      CHARSET[idx] as char
    })
    .collect();

  salt
}

pub fn get_salted_password(password: &str, salt: &str) -> String {
  let hash = Sha256::new()
    .chain_update(password)
    .chain_update(salt)
    .finalize();

  hash
    .into_iter()
    .map(|b| { format!("{:x?}", b) })
    .collect::<Vec<String>>()
    .join("")
}

pub fn generate_jwt(user_id: &str, username: &str) -> String {
  let claims = Claims::new(String::from(user_id), String::from(username));
  let secret = get_secret();
  encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).expect("jwt token to be generated")
}

#[allow(dead_code)]
fn validate_and_decode(token: &str) -> Option<Claims> {
  let token = decode::<Claims>(token, &DecodingKey::from_secret(get_secret().as_ref()), &Validation::default());
  let claims = token.and_then(|td| Ok(td.claims));

  match claims {
    Ok(claims) => {
      if claims.iss == *ISSUER {
        Some(claims)
      } else {
        None
      }
    },
    _ => None
  }
}

pub fn get_secret() -> String {
  env::var("JWT_SECRET").expect("JWT_SECRET must be set")
}

pub fn user_action_authorization(user_id: i64, auth_id: i64, msg: &str) -> Result<(), AppError> {
  if user_id != auth_id {
    return Err(AppError::Forbidden(String::from(msg)));
  }
  Ok(())
}

pub async fn event_action_authorization(pool: &DbState, event_id: i64, auth_id: i64, msg: &str) -> Result<(), AppError> {
  let event_to_update = sqlx::query!("SELECT creator FROM event WHERE id = ?1", event_id)
    .fetch_one(pool)
    .await?;
  if event_to_update.creator != auth_id {
    return Err(AppError::Forbidden(String::from(msg)));
  }
  Ok(())
}

pub async fn requirement_action_authorization(pool: &DbState, req_id: i64, auth_id: i64, msg: &str) -> Result<(), AppError> {
  let event_to_update = sqlx::query!("SELECT creator FROM event WHERE id = (SELECT event FROM requirement where id = ?1)", req_id)
    .fetch_one(pool)
    .await?;
  if event_to_update.creator != auth_id {
    return Err(AppError::Forbidden(String::from(msg)));
  }
  Ok(())
}
