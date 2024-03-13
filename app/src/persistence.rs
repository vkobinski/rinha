use sqlx::{postgres::{PgAdvisoryLock, PgPoolOptions}, PgConnection, PgPool};
use std::error::Error;

use crate::models::transacao::{Descricao, NewTransacao, TipoTransacao};

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

        //isabella <3
        //Meu amoire

        Ok(PostgresRepository{pool})
    }

    pub async fn warm_up(&self) {

        for _i in 0..50 {
            let _ = self.find_cliente_by_id(9999).await;
        }

        for _i in 0..50 {
            let transacao = NewTransacao{
                valor : 2000,
                tipo : TipoTransacao::CREDITO,
                descricao : Descricao::try_from(String::from("teste")).unwrap()
            };
            let _ = self.create_transacao(transacao, 9999).await;
        }



    }
}
