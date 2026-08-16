use crate::DbPool;

#[derive(Clone)]
pub(crate) struct UserRepository {
    pool: DbPool,
}

impl UserRepository {
    pub(crate) fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}
