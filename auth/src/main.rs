mod repository;
mod schema;

use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use diesel::{Insertable, Queryable, Selectable};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use repository::{UserRepository, UserRepositoryError};
use schema::users;
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_scalar::{Scalar, Servable};

const DEFAULT_DATABASE_URL: &str = "postgres://auth:auth@localhost:5432/auth";
type DbPool = Pool<AsyncPgConnection>;

// -------------------------------- Application State --------------------------------

#[derive(Clone)]
struct AppState {
    user_repository: UserRepository,
}

// -------------------------------- Request / Response --------------------------------

#[derive(Deserialize, Insertable, ToSchema)]
#[diesel(table_name = users)]
struct CreateUser {
    username: String,
}

#[derive(Queryable, Selectable, Serialize, ToSchema)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct User {
    id: i64,
    username: String,
}

// -------------------------------- Handlers --------------------------------

async fn root() -> &'static str {
    "Hello, World!"
}

#[utoipa::path(
    post,
    path = "/users",
    request_body = CreateUser,
      responses(
          (status = 201, description = "사용자 생성 성공", body = User),
          (status = 409, description = "이미 존재하는 사용자 이름"),
          (status = 500, description = "서버 내부 오류")
      )
)]
async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> Result<(StatusCode, Json<User>), StatusCode> {
    let user = state
        .user_repository
        .create(&payload)
        .await
        .map_err(repository_error_status)?;

    Ok((StatusCode::CREATED, Json(user)))
}

#[utoipa::path(
    get,
    path = "/users",
    responses(
        (status = 200, description = "사용자 목록 조회 성공", body = [User]),
        (status = 500, description = "서버 내부 오류")
    )
)]
async fn list_users(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<Vec<User>>), StatusCode> {
    let users = state
        .user_repository
        .list()
        .await
        .map_err(repository_error_status)?;

    Ok((StatusCode::OK, Json(users)))
}

// -------------------------------- Error Handling --------------------------------

fn repository_error_status(error: UserRepositoryError) -> StatusCode {
    match error {
        UserRepositoryError::UsernameAlreadyExists => StatusCode::CONFLICT,
        UserRepositoryError::Internal(error) => {
            eprintln!("user repository error: {error}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

// -------------------------------- Database --------------------------------

fn create_database_pool() -> DbPool {
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
    let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    Pool::builder(manager)
        .max_size(16)
        .build()
        .expect("failed to create Diesel connection pool")
}

// -------------------------------- API Documentation --------------------------------

#[derive(OpenApi)]
#[openapi(paths(create_user, list_users), components(schemas(CreateUser, User)))]
struct ApiDoc;

// -------------------------------- Application Entry Point --------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let pool = create_database_pool();
    let connection = pool
        .get()
        .await
        .expect("failed to connect to PostgreSQL at startup");
    drop(connection);
    let state = AppState {
        user_repository: UserRepository::new(pool),
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/users", get(list_users).post(create_user))
        .merge(Scalar::with_url("/docs", ApiDoc::openapi()))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Root: http://localhost:3000");
    println!("API docs: http://localhost:3000/docs");

    axum::serve(listener, app).await.unwrap();
}
