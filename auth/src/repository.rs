use std::error::Error;

use diesel::{QueryDsl, SelectableHelper};
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::deadpool::Object};

use crate::{CreateUser, DbPool, User, schema::users};

type DbConnection = Object<AsyncPgConnection>;

#[derive(Clone)]
pub(crate) struct UserRepository {
    pool: DbPool,
}

impl UserRepository {
    pub(crate) fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub(crate) async fn create(&self, payload: &CreateUser) -> Result<User, UserRepositoryError> {
        let mut connection = self.connection().await?;

        diesel::insert_into(users::table)
            .values(payload)
            .returning(User::as_returning())
            .get_result(&mut connection)
            .await
            .map_err(|error| match error {
                diesel::result::Error::DatabaseError(
                    diesel::result::DatabaseErrorKind::UniqueViolation,
                    _,
                ) => UserRepositoryError::UsernameAlreadyExists,
                error => UserRepositoryError::Internal(Box::new(error)),
            })
    }

    pub(crate) async fn list(&self) -> Result<Vec<User>, UserRepositoryError> {
        let mut connection = self.connection().await?;

        users::table
            .select(User::as_select())
            .load(&mut connection)
            .await
            .map_err(|error| UserRepositoryError::Internal(Box::new(error)))
    }

    async fn connection(&self) -> Result<DbConnection, UserRepositoryError> {
        self.pool
            .get()
            .await
            .map_err(|error| UserRepositoryError::Internal(Box::new(error)))
    }
}

pub(crate) enum UserRepositoryError {
    UsernameAlreadyExists,
    Internal(Box<dyn Error + Send + Sync>),
}
