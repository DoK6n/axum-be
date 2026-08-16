mod repository;
mod schema;

use axum::{Router, routing::get};
use diesel::{Insertable, Queryable, Selectable};
use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, deadpool::Pool},
};
use repository::UserRepository;
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

    let app = Router::new().route("/", get(root)).with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Root: http://localhost:3000");

    axum::serve(listener, app).await.unwrap();
}
