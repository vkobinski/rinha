use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{error::Error};

#[derive(Debug)]
pub enum PersistenceError {
    UniqueViolation,
    NotEnoughFunds,
    IdDoesNotExist,
    DatabaseError(Box<dyn Error + Send + Sync>),
}

pub struct PostgresRepository {
    pub pool: PgPool,
}

pub type PersistenceResult<T> = Result<T, PersistenceError>;

impl From<sqlx::Error> for PersistenceError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(err) if err.is_unique_violation() => {
                PersistenceError::UniqueViolation
            },
            sqlx::Error::RowNotFound => {
                PersistenceError::IdDoesNotExist
            },
            _ => PersistenceError::DatabaseError(Box::new(error)),
        }

    }

}

impl PostgresRepository {

    pub async fn connect(url: &str, pool_size: u32) -> Result<Self, sqlx::Error> {

        let pool = PgPoolOptions::new()
            .max_connections(pool_size)
            .connect(url)
            .await?;

        Ok(PostgresRepository{pool})
    }
}
