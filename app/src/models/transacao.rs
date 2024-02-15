use ::chrono::{DateTime, Utc};
use redis::Commands;
use serde::{Serialize, Deserialize};
use sqlx::{postgres::{PgAdvisoryLock, PgAdvisoryLockKey}, Acquire, Connection, Executor, FromRow, PgConnection, PgPool};

use crate::persistence::{PersistenceError, PersistenceResult, PostgresRepository};

#[derive(Serialize, Deserialize, Clone, sqlx::Type)]
#[sqlx(type_name = "VARCHAR")]
pub enum TipoTransacao {
    #[serde(rename = "c")]
    #[sqlx(rename = "c")]
    CREDITO,
    #[serde(rename = "d")]
    #[sqlx(rename = "d")]
    DEBITO,
    ERROR,
}

impl<'a> Into<&'a str> for TipoTransacao {
    fn into(self) -> &'a str {
        match self {
            Self::CREDITO => "c",
            Self::DEBITO => "d",
            _ => unreachable!("Invalid TipoTransacao")

        }
    }
}

impl From<&str> for TipoTransacao {
    fn from(value: &str) -> Self {
        match value {
            "c" => Self::CREDITO,
            "d" => Self::DEBITO,
            _ => Self::ERROR

        }
    }
}

#[derive(Clone, Serialize, Deserialize, FromRow)]
pub struct Transacao {
    #[serde(skip_serializing)]
    pub transacao_id: i32,
    pub valor: i32,
    pub tipo: TipoTransacao,
    pub descricao: Descricao,
    pub realizada_em: DateTime<Utc>,
}

#[derive(Clone, Deserialize)]
pub struct NewTransacao {
    pub valor: i32,
    pub tipo: TipoTransacao,
    pub descricao: Descricao,
}

#[derive(Serialize)]
pub struct TransacaoResponse {
    pub limite: i32,
    pub saldo: i32,
}

impl PostgresRepository {

    pub async fn create_transacao(&self, new_transacao: NewTransacao, id: i32) -> PersistenceResult<TransacaoResponse> {

        let mut transaction = self.pool.begin().await.expect("Acquiring lock");
        let novo_saldo : TransacaoResponse;
        let saldo = self.find_saldo_by_cliente_id_with_lock(id, &mut transaction).await;

        match saldo {
            Ok(Some(saldo)) => {

                let novo_total: i32;

                match new_transacao.tipo {
                    TipoTransacao::CREDITO => {
                        novo_total = saldo.total + new_transacao.valor;
                    },
                    TipoTransacao::DEBITO => {
                    if (saldo.total - new_transacao.valor) < -saldo.limite {
                        return Err(PersistenceError::NotEnoughFunds)
                    }
                        novo_total = saldo.total - new_transacao.valor;
                    },
                    TipoTransacao::ERROR => {
                        return Err(PersistenceError::NotEnoughFunds)
                    }
                };

            let datetime: DateTime<Utc> = Utc::now().to_utc();
        let tipo_str : & str = new_transacao.tipo.into();

        let transacao_insert = sqlx::query!(
            "
            INSERT INTO transacao (cliente_id, valor, tipo, descricao, realizada_em)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING cliente_id
            ",
            id,
            new_transacao.valor,
            tipo_str,
            new_transacao.descricao.as_str(),
            datetime
            )
            .fetch_one(&mut *transaction)
            .await
            .map(|row| Ok::<i32, PersistenceError>(row.cliente_id))
            .map_err(PersistenceError::from);


                let saldo_id : Result<i32, PersistenceError> = sqlx::query!(
                    "
                    UPDATE saldo
                    SET total = $1
                    WHERE saldo_id = $2
                    RETURNING saldo_id
                    ",
                    novo_total,
                    saldo.saldo_id,
                    )
                    .fetch_one(&mut *transaction)
                    .await
                    .map(|row| Ok(row.saldo_id))
                    .map_err(PersistenceError::from)?;


                match saldo_id {
                    Ok(_) => {
                        novo_saldo = TransacaoResponse{limite: saldo.limite, saldo: novo_total}
                    }
                    Err(err) => {
                        return Err(PersistenceError::from(err))
                    }
                };
            },
            Ok(None) => {
                return Err(PersistenceError::IdDoesNotExist)
            },
            Err(err) => {
                       return Err(PersistenceError::from(err));
            }
        };

        transaction.commit().await?;

        Ok(novo_saldo)

    }

    pub async fn find_transacoes_by_cliente_id(&self, id: i32) -> PersistenceResult<Vec<Transacao>> {


        let res = sqlx::query_as(
            "
            SELECT transacao_id, valor, tipo, descricao, realizada_em
            FROM transacao
            WHERE cliente_id = $1
            ORDER BY realizada_em DESC
            LIMIT 10
            FOR UPDATE
            "
            )
            .bind(id)
            .fetch_all(&self.pool)
            .await
            .map_err(PersistenceError::from);


        res
    }

}

macro_rules! new_string_type {
    ($type:ident, max_length = $max_length:expr, error = $error_message:expr, min_length = $min_length:expr, error_min = $error_min:expr) => {
        #[derive(Clone, Serialize, Deserialize, FromRow, sqlx::Type)]
        #[serde(try_from = "String")]
        #[sqlx(type_name = "VARCHAR")]
        pub struct $type(String);

        impl $type {
            pub fn as_str(&self) -> &str {
                &self.0
            }

        }

        impl TryFrom<String> for $type {
            type Error = &'static str;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.len() < $min_length {
                    Err($error_min)
                }
                else if value.len() <= $max_length {
                    Ok($type(value))
                } else {
                    Err($error_message)
                }

            }
        }

        impl From<$type> for String {
            fn from(value: $type) -> Self {
                value.0
            }

        }

    };
}

new_string_type!(Descricao, max_length = 10, error = "descricao is too big", min_length = 1, error_min = "descricao is too short");
