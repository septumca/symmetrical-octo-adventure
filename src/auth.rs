use axum::{http::{StatusCode}, async_trait, extract::{FromRequest, RequestParts}};
use chrono::{Utc, Duration};
use jsonwebtoken::{EncodingKey, Header, encode, decode, DecodingKey, Validation};
use rand::Rng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::{env};

pub const ISSUER: &str = "zmtwc";

pub struct UserAuth(pub i64);

#[async_trait]
impl<B> FromRequest<B> for UserAuth
where
    B: Send,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
      let token = req.headers()
        .get("X-JWT-Token")
        .and_then(|header| header.to_str().ok());

      if let Some(token) = token {
        if let Some(claims) = validate_and_decode(token) {
          return Ok(UserAuth(claims.sub.parse::<i64>().unwrap()));
        } else {
          return Err((StatusCode::UNAUTHORIZED, "Invalid JWT token"))
        }
      }
      Err((StatusCode::UNAUTHORIZED, "`X-JWT-Token` header is missing"))
    }
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
