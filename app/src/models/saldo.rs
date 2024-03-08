use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use sqlx::{FromRow, Postgres, Transaction};

use crate::persistence::{PersistenceError, PersistenceResult, PostgresRepository};

#[derive(Clone, Serialize, Deserialize, FromRow)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Saldo {
    #[serde(skip_serializing)]
    pub saldo_id: i32,
    pub total: i32,
    pub limite: i32,
    #[serde(skip_deserializing)]
    #[sqlx(skip)]
    pub data_extrato: DateTime<Utc>
}

#[derive(Clone, Deserialize)]
pub struct NewSaldo {
    pub total: i32,
    pub limite: i32,
}


impl PostgresRepository {
    pub async fn find_saldo_by_cliente_id_with_lock(&self, cliente_id: i32, transaction: &mut Transaction<'_, Postgres>) -> PersistenceResult<Option<Saldo>> {

        let result = sqlx::query_as(
            "
            SELECT saldo_id, total, limite
            FROM saldo
            WHERE cliente_id = $1
            FOR UPDATE
            "
            ).bind(cliente_id)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(PersistenceError::from);

        result
    }

pub async fn find_saldo_by_cliente_id(&self, cliente_id: i32) -> PersistenceResult<Option<Saldo>> {

        let result = sqlx::query_as(
            "
            SELECT saldo_id, total, limite
            FROM saldo
            WHERE cliente_id = $1
            "
            ).bind(cliente_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(PersistenceError::from);

        result
    }



    pub async fn create_saldo(&self, new_saldo: &NewSaldo, cliente_id: i32) -> PersistenceResult<i32> {

        sqlx::query!(
            "
            INSERT into saldo (cliente_id, total, limite)
            VALUES ($1, $2, $3)
            RETURNING saldo_id
            ",
            cliente_id,
            new_saldo.total,
            new_saldo.limite
        )
            .fetch_one(&self.pool)
            .await
            .map(|row| Ok(row.saldo_id))
            .map_err(PersistenceError::from)?

    }

}
