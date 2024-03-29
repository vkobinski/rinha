use ::chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::time::Instant;
use crate::{models::saldo::NewSaldo, persistence::{PersistenceError, PersistenceResult, PostgresRepository}};

#[derive(Serialize, Deserialize, Clone, sqlx::Type, Debug)]
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
            _ => unreachable!("Invalid TipoTransacao"),
        }
    }
}

impl From<&str> for TipoTransacao {
    fn from(value: &str) -> Self {
        match value {
            "c" => Self::CREDITO,
            "d" => Self::DEBITO,
            _ => Self::ERROR,
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
     pub async fn create_transacao_in_db(
        &self,
        new_transacao: NewTransacao,
        id: i32,
    ) -> PersistenceResult<TransacaoResponse> {


                let tipo_str: &str = new_transacao.tipo.into();
                let mut novo_saldo: Option<TransacaoResponse> = None;

                let saldo = sqlx::query!(
                    "
                    SELECT add_transaction($1, $2, $3, $4)
                    "
                    ,id,
                    new_transacao.valor,
                    tipo_str,
                    new_transacao.descricao.as_str()
                    )
                .fetch_optional(&self.pool)
                .await
                .map_err(PersistenceError::from);


                match saldo {
                    Ok(Some(c)) => {
                        let details = match c.add_transaction {
                              Some(detail) => {
                                  detail
                              },
                              None => {
                                  return Err(PersistenceError::IdDoesNotExist)
                              }

                          };
                          let saida : NewSaldo = match serde_json::from_value(details.clone()) {
                              Ok(s) => s,
                              Err(_) => {
                                  return Err(PersistenceError::NotEnoughFunds)
                              }
                          };

                        novo_saldo = Some(TransacaoResponse {
                            limite: saida.limite,
                            saldo: saida.total,
                        })
                    },
                    Ok(_) => {
                    },
                    Err(err) => {
                        return Err(PersistenceError::from(err))},
                };

                match novo_saldo {
                    Some(s) => Ok(s),
                    None => {
                        return Err(PersistenceError::IdDoesNotExist)
                    },

                }

    }


    pub async fn create_transacao(
        &self,
        new_transacao: NewTransacao,
        id: i32,
    ) -> PersistenceResult<TransacaoResponse> {


        let mut transaction = self.pool.begin().await.expect("Acquiring lock");
        let novo_saldo: TransacaoResponse;
        let saldo = self
            .find_saldo_by_cliente_id_with_lock(id, &mut transaction)
            .await;

        match saldo {
            Ok(Some(saldo)) => {
                let novo_total: i32;

                match new_transacao.tipo {
                    TipoTransacao::CREDITO => {
                        novo_total = saldo.total + new_transacao.valor;
                    }
                    TipoTransacao::DEBITO => {
                        if (saldo.total - new_transacao.valor) < -saldo.limite {
                            return Err(PersistenceError::NotEnoughFunds);
                        }
                        novo_total = saldo.total - new_transacao.valor;
                    }
                    TipoTransacao::ERROR => return Err(PersistenceError::NotEnoughFunds),
                };

                let tipo_str: &str = new_transacao.tipo.into();


                let saldo_id = sqlx::query!(
                    "
                    SELECT create_transaction($1, $2, $3, $4, $5, $6)
                    ",
                    id,
                    new_transacao.valor,
                    tipo_str,
                    new_transacao.descricao.as_str(),
                    novo_total,
                    saldo.saldo_id
                    )
                .fetch_one(&mut *transaction)
                .await
                .map_err(PersistenceError::from);


                tokio::spawn(transaction.commit());

                match saldo_id {
                    Ok(_) => {
                        novo_saldo = TransacaoResponse {
                            limite: saldo.limite,
                            saldo: novo_total,
                        }
                    }
                    Err(err) => return Err(PersistenceError::from(err)),
                };
            }
            Ok(None) => return Err(PersistenceError::IdDoesNotExist),
            Err(err) => {
                return Err(PersistenceError::from(err));
            }
        };

        Ok(novo_saldo)
    }

    pub async fn find_transacoes_by_cliente_id(
        &self,
        id: i32,
    ) -> PersistenceResult<Vec<Transacao>> {
        let res = sqlx::query_as(
            "
            SELECT transacao_id, valor, tipo, descricao, realizada_em
            FROM transacao
            WHERE cliente_id = $1
            ORDER BY realizada_em DESC
            LIMIT 10
            ",
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
                } else if value.len() <= $max_length {
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

new_string_type!(
    Descricao,
    max_length = 10,
    error = "descricao is too big",
    min_length = 1,
    error_min = "descricao is too short"
);
