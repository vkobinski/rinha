use axum::http::Result;

use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::persistence::{PersistenceError, PersistenceResult, PostgresRepository};

use super::{saldo::{NewSaldo, Saldo}, transacao::Transacao};

#[derive(Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Cliente {
    #[serde(skip_serializing)]
    pub cliente_id: i32,
    pub saldo: Saldo,
    pub ultimas_transacoes: Option<Vec<Transacao>>
}

impl PostgresRepository {

    pub async fn find_cliente_by_id(&self, id: i32) -> PersistenceResult<Option<Cliente>> {

        let cliente_saldo = self.find_saldo_by_cliente_id(id).await;
        let cliente_transacoes = self.find_transacoes_by_cliente_id(id).await;

        match(cliente_saldo, cliente_transacoes) {
            (Ok(Some(mut cliente_saldo)), Ok(cliente_transacoes)) => {

                cliente_saldo.data_extrato = Utc::now();

                Ok(Some(Cliente {
                    cliente_id: id,
                    saldo: cliente_saldo,
                    ultimas_transacoes: Some(cliente_transacoes)
            }))
            },
            (Ok(None), Ok(_)) => {
                Err(PersistenceError::IdDoesNotExist)
            },
            ((_, Err(e)) | (Err(e), _)) => {
                eprintln!("{:?}", e);
                Err(PersistenceError::UniqueViolation)
            }
        }

    }

    pub async fn create_cliente(&self, limite: i32) -> PersistenceResult<i32> {

        let insert_cliente = sqlx::query!("INSERT INTO cliente DEFAULT VALUES RETURNING cliente_id")
            .fetch_one(&self.pool)
            .await
            .map(|row| row.cliente_id)
            .map_err(PersistenceError::from)?;

        match insert_cliente {
            cliente => {
                match self.create_saldo(&NewSaldo{limite, total: 0}, cliente).await {
                    Ok(saldo) => {
                        Ok(saldo)
                    }
                    Err(error) => {
                        Err(error)
                    }
                }
            }
        }

    }
}
