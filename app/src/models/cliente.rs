use chrono::Utc;
use serde::{Serialize, Deserialize};
use sqlx::postgres::PgRow;

use crate::{models::transacao, persistence::{PersistenceError, PersistenceResult, PostgresRepository}};

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

        let cliente = sqlx::query!(
            "
            SELECT get_cliente_details($1)
            "
            , id
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(PersistenceError::from);

        match cliente {
            Ok(Some(c)) => {

                let details = match c.get_cliente_details {
                    Some(detail) => {
                        detail
                    },
                    None => {
                        return Err(PersistenceError::IdDoesNotExist)
                    }

                };
                let mut saida : Cliente = serde_json::from_value(details.clone()).unwrap();

                saida.saldo.data_extrato = Utc::now();

                Ok(Some(saida))

            },
            Ok(None) => {
                Err(PersistenceError::IdDoesNotExist)
            },
            Err(_) => {
                Err(PersistenceError::UniqueViolation)
            },

        }

//
//        match(saldo, transacao) {
//            (Ok(Some(mut cliente_saldo)), Ok(cliente_transacoes)) => {
//
//                cliente_saldo.data_extrato = Utc::now();
//
//                Ok(Some(Cliente {
//                    cliente_id: id,
//                    saldo: cliente_saldo,
//                    ultimas_transacoes: Some(cliente_transacoes)
//            }))
//            },
//            (Ok(None), Ok(_)) => {
//                Err(PersistenceError::IdDoesNotExist)
//            },
//            (_, Err(e)) | (Err(e), _) => {
//                eprintln!("{:?}", e);
//                Err(PersistenceError::UniqueViolation)
//            }
//        }

    }

    #[allow(dead_code)]
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
