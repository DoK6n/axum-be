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

const DEFAULT_DATABASE_URL: &str = "postgres://auth:auth@localhost:5432/auth";
type DbPool = Pool<AsyncPgConnection>;

// -------------------------------- Application State --------------------------------

#[derive(Clone)]
struct AppState {
    user_repository: UserRepository,
}

// -------------------------------- Request / Response --------------------------------

#[derive(Deserialize, Insertable)]
#[diesel(table_name = users)]
struct CreateUser {
    username: String,
}

#[derive(Queryable, Selectable, Serialize)]
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
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Root: http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
