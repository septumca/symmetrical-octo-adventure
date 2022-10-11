use axum::{http::{Request, StatusCode}, middleware::Next, response::IntoResponse};
use chrono::{Utc, Duration};
use jsonwebtoken::{EncodingKey, Header, encode, decode, DecodingKey, Validation};
use rand::Rng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::{env};

pub const ISSUER: &str = "zmtwc";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
  iss: String,
  sub: String,
  exp: i64,
  iat: i64,
}

impl Claims {
  pub fn new(sub: String) -> Self {
    let iat = Utc::now();
    let exp = iat + Duration::hours(24);

    Self {
      iss: String::from(ISSUER),
      sub,
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

pub fn generate_jwt(user_id: String) -> String {
  let claims = Claims::new(user_id);
  let secret = get_secret();
  encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).expect("jwt token to be generated")
}

#[allow(dead_code)]
fn token_is_valid(token: String) -> bool {
  let token = decode::<Claims>(&token, &DecodingKey::from_secret(get_secret().as_ref()), &Validation::default());
  token.is_ok() && token.expect("token should be valid").claims.iss == *ISSUER
}

#[cfg(debug_assertions)]
pub async fn auth<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
  next.run(req).await
}

#[allow(dead_code)]
#[cfg(not(debug_assertions))]
pub async fn auth<B>(req: Request<B>, next: Next<B>) -> impl IntoResponse {
  let token = req.headers()
    .get("X-JWT-Token")
    .and_then(|header| header.to_str().ok());

  match token {
    Some(token) if token_is_valid(String::from(token)) => {
      Ok(next.run(req).await)
    }
    _ => Err(StatusCode::UNAUTHORIZED),
  }
}

#[cfg(debug_assertions)]
pub fn get_secret() -> String {
  String::from("mysecret")
}

#[allow(dead_code)]
#[cfg(not(debug_assertions))]
pub fn get_secret() -> String {
  env::var("JWT_SECRET").expect("JWT_SECRET must be set")
}