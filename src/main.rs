use axum::{
  http::Method,
  routing::{get, post, put, delete},
  Router, Extension, middleware,
};
use dotenvy::dotenv;
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::{shutdown_signal};
use user::{authentificate};
use std::{env};
use std::net::SocketAddr;
use sqlx::{SqlitePool, Pool, Sqlite};
use db_modeling::{database_down, database_up, database_fill};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tower::ServiceBuilder;

mod error;
mod utils;
mod auth;
mod db_modeling;
mod user;
mod event;
mod participant;
mod requirement;
mod fullfillment;

type DbState = Pool<Sqlite>;

#[allow(dead_code)]
async fn app(pool: Pool<Sqlite>) -> Router {
  let cors = CorsLayer::new()
    .allow_methods(vec![Method::GET, Method::POST, Method::DELETE, Method::PUT])
    .allow_headers(Any)
    .allow_origin(Any);

  let public = Router::new()
    .route("/up", get(database_up))
    .route("/down", get(database_down))
    .route("/fill", get(database_fill))
    .route("/register", post(user::create))
    .route("/authentificate", post(authentificate))
    .route("/event", get(event::all))
    .route("/event/:id", get(event::single))
    .route("/user", get(user::all))
    ;

  let private = Router::new()
    .route("/user/:id", get(user::single))
    .route("/user/:id", put(user::update))
    .route("/user/:id", delete(user::delete))

    .route("/event", post(event::create))
    .route("/event/:id", put(event::update))
    .route("/event/:id", delete(event::delete))

    .route("/participant", post(participant::create))
    .route("/participant/:user_id/:event_id", delete(participant::delete))

    .route("/requirement", post(requirement::create))
    .route("/requirement/:id", put(requirement::update))
    .route("/requirement/:id", delete(requirement::delete))

    .route("/fullfillment", post(fullfillment::create))
    .route("/fullfillment/:user_id/:requirement_id", delete(fullfillment::delete))

    .route_layer(middleware::from_fn(auth::auth))
    ;

  Router::new()
    .merge(public)
    .merge(private)
    .layer(
      ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(Extension(pool))
    )
}



#[tokio::main]
async fn main() {
  dotenv().ok();
  let filter = EnvFilter::from_default_env();
  let filtered_layer = tracing_subscriber::fmt::layer().with_filter(filter);
  tracing_subscriber::registry()
    .with(filtered_layer)
    .init();

  for (key, value) in env::vars() {
    tracing::debug!("{key}: {value}");
  }

  let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();
  let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
  tracing::info!("listening on {}", addr);
  axum::Server::bind(&addr)
    .serve(app(pool).await.into_make_service())
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}

// https://github.com/tokio-rs/axum/tree/0.5.x/examples/testing
#[cfg(test)]
mod main {

  use super::*;
  use axum::{
      body::Body,
      http::{self, Request, StatusCode},
  };
  use serde_json::{json, Value};
  use tower::ServiceExt; // for `app.oneshot()`

  async fn test_api_fail(app: Router, uri: &str, method: Method, body: Option<Value>, expected_status: StatusCode) {
    test_api_base(app, uri, method, body, None, expected_status).await
  }

  async fn test_api_success(app: Router, uri: &str, method: Method, body: Option<Value>, expected_response: Option<Value>) {
    test_api_base(app, uri, method, body, expected_response, StatusCode::OK).await
  }

  async fn test_api_base(app: Router, uri: &str, method: Method, body: Option<Value>, expected_response: Option<Value>, expected_status: StatusCode) {
    let body = body.and_then(|b| Some(Body::from(serde_json::to_vec(&b).unwrap()))).unwrap_or(Body::empty());
    let response = app
      .oneshot(
          Request::builder()
            .method(method)
            .uri(uri)
            .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
            .body(body)
            .unwrap(),
      )
      .await
      .unwrap();

    assert_eq!(response.status(), expected_status);

    if let Some(expected_response) = expected_response {
      let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
      let body: Value = serde_json::from_slice(&body).unwrap();
      assert_eq!(body, expected_response);
    }
  }



  async fn setup() -> (Router, SqlitePool) {
    env::set_var("DATABASE_URL", "sqlite::memory:");
    let pool = SqlitePool::connect(&env::var("DATABASE_URL").unwrap()).await.unwrap();
    (app(pool.clone()).await, pool)
  }

  async fn setup_with_structure() -> (Router, SqlitePool) {
    let (app, pool) = setup().await;
    let _ = sqlx::query_file!("./sql/up.sql").execute(&pool).await.unwrap();
    (app, pool)
  }

  async fn setup_with_data() -> (Router, SqlitePool) {
    let (app, pool) = setup_with_structure().await;
    let _ = sqlx::query_file!("./sql/test.sql").execute(&pool).await.unwrap();
    (app, pool)
  }

  #[tokio::test]
  async fn db_up() {
    let (app, _) = setup().await;
    test_api_success(app, "/up", http::Method::GET, None, None).await;
  }

  #[tokio::test]
  async fn db_down() {
    let (app, _) = setup_with_structure().await;
    test_api_success(app, "/down", http::Method::GET, None, None).await;
  }

  mod user {
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

      test_api_success(app, "/register", http::Method::POST, Some(body_json), Some(expected_response)).await;
    }

    #[tokio::test]
    async fn single() {
      let (app, _) = setup_with_data().await;
      let expected_response = json!({
        "id": 1,
        "username": "username1"
      });

      test_api_success(app, "/user/1", http::Method::GET, None, Some(expected_response)).await;
    }

    #[tokio::test]
    async fn all() {
      let (app, _) = setup_with_data().await;
      let expected_response = json!([
        {
          "id": 1,
          "username": "username1"
        },
        {
          "id": 2,
          "username": "username2"
        },
        {
          "id": 3,
          "username": "username3"
        },
        {
          "id": 4,
          "username": "username4"
        },
        {
          "id": 5,
          "username": "username5"
        },
        {
          "id": 6,
          "username": "username6"
        }
      ]);

      test_api_success(app, "/user", http::Method::GET, None, Some(expected_response)).await;
    }

    #[tokio::test]
    async fn update() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "username": "edited_username",
      });

      test_api_success(app, "/user/1", http::Method::PUT, Some(body_json), None).await;

      let result = sqlx::query!("select * from user where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.username, "edited_username");
    }

    #[tokio::test]
    async fn delete_simple() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/user/5", http::Method::DELETE, None, None).await;

      let results = sqlx::query!("select * from user")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 5);
      assert_eq!(results[0].id, 1);
      assert_eq!(results[1].id, 2);
      assert_eq!(results[2].id, 3);
      assert_eq!(results[3].id, 4);
      assert_eq!(results[4].id, 6);
    }

    #[tokio::test]
    async fn delete_with_fk() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/user/1", http::Method::DELETE, None, None).await;

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
  }

  mod event {
    use super::*;

    #[tokio::test]
    async fn create() {
      let (app, pool) = setup_with_structure().await;
      let body_json = json!({
        "name": "my new event",
        "description": "my event description",
        "creator": 1
      });
      let expected_response = json!({
        "id": 1,
        "name": "my new event",
        "description": "my event description",
        "creator": {
          "id": 1,
          "username": "username1"
        }
      });
      let _ = sqlx::query!("INSERT INTO user (id, username, password, salt) VALUES (1, 'username1', 'sha256password', 'somesalt')")
        .execute(&pool)
        .await
        .unwrap();

      test_api_success(app, "/event", http::Method::POST, Some(body_json), Some(expected_response)).await;
    }

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
        }
      });

      test_api_success(app, "/event/1", http::Method::GET, None, Some(expected_response)).await;
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
          }
        },
        {
          "id": 2,
          "name": "event-2",
          "description": "some description 2",
          "creator": {
            "id": 6,
            "username": "username6"
          }
        },
        {
          "id": 3,
          "name": "event-3",
          "description": "some description 3",
          "creator": {
            "id": 4,
            "username": "username4"
          }
        }
      ]);

      test_api_success(app, "/event", http::Method::GET, None, Some(expected_response)).await;
    }

    #[tokio::test]
    async fn update() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "name": "edited name event 1",
        "description": "some description 1",
      });

      test_api_success(app, "/event/1", http::Method::PUT, Some(body_json), None).await;

      let result = sqlx::query!("select * from event where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.name, "edited name event 1");
      assert_eq!(result.description, Some("some description 1".to_owned()));
    }

    #[tokio::test]
    async fn delete_simple() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/event/3", http::Method::DELETE, None, None).await;

      let results = sqlx::query!("select * from event")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 2);
      assert_eq!(results[0].id, 1);
      assert_eq!(results[0].name, "event-1");
      assert_eq!(results[1].id, 2);
      assert_eq!(results[1].name, "event-2");
    }

    #[tokio::test]
    async fn delete_with_fk() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/event/1", http::Method::DELETE, None, None).await;

      let results = sqlx::query!("select * from event")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 2);
      assert_eq!(results[0].id, 2);
      assert_eq!(results[0].name, "event-2");
      assert_eq!(results[1].id, 3);
      assert_eq!(results[1].name, "event-3");

      //TODO cleanup checks
    }
  }

  mod participant {
    use super::*;

    #[tokio::test]
    async fn create() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "event": 3,
        "user": 6,
      });

      test_api_success(app, "/participant", http::Method::POST, Some(body_json), None).await;

      let results = sqlx::query!("select * from participant where event = 3 and user = 6")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn delete() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/participant/3/1", http::Method::DELETE, None, None).await;

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

  mod requirement {
    use super::*;

    #[tokio::test]
    async fn create_default_size() {
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

      test_api_success(app, "/requirement", http::Method::POST, Some(body_json), Some(expected_response)).await;
    }

    #[tokio::test]
    async fn create_custom_size() {
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

      test_api_success(app, "/requirement", http::Method::POST, Some(body_json), Some(expected_response)).await;
    }

    #[tokio::test]
    async fn update() {
      let (app, pool) = setup_with_data().await;
      let body_json = json!({
        "name": "edited req name",
        "description": "some other description 1",
      });

      test_api_success(app, "/requirement/1", http::Method::PUT, Some(body_json), None).await;

      let result = sqlx::query!("select * from requirement where id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();

      assert_eq!(result.name, "edited req name");
      assert_eq!(result.description, Some("some other description 1".to_owned()));
    }

    #[tokio::test]
    async fn delete_simple() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/requirement/2", http::Method::DELETE, None, None).await;

      let results = sqlx::query!("select id from requirement")
        .fetch_all(&pool)
        .await
        .unwrap();

      assert_eq!(results.len(), 2);
      assert_eq!(results[0].id, 1);
      assert_eq!(results[1].id, 3);
    }

    #[tokio::test]
    async fn delete_with_fk() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/requirement/1", http::Method::DELETE, None, None).await;

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
  }

  mod fullfillment {
    use super::*;

    #[tokio::test]
    async fn create_to_single() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 2,
        "user": 6,
      });

      test_api_success(app, "/fullfillment", http::Method::POST, Some(body_json), None).await;
    }

    #[tokio::test]
    async fn create_to_mutliple() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 1,
        "user": 6,
      });

      test_api_success(app, "/fullfillment", http::Method::POST, Some(body_json), None).await;
    }

    #[tokio::test]
    async fn create_to_non_existing() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 5,
        "user": 6,
      });

      test_api_fail(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::NOT_FOUND).await;
    }

    #[tokio::test]
    async fn create_fully_assigned() {
      let (app, _) = setup_with_data().await;
      let body_json = json!({
        "requirement": 3,
        "user": 6,
      });

      test_api_fail(app, "/fullfillment", http::Method::POST, Some(body_json), StatusCode::INTERNAL_SERVER_ERROR).await;
    }

    #[tokio::test]
    async fn delete() {
      let (app, pool) = setup_with_data().await;

      test_api_success(app, "/fullfillment/4/1", http::Method::DELETE, None, None).await;

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