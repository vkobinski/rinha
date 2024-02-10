use axum::{Router, routing::{get, post}, response::IntoResponse, Json, extract::{Path, State}, http::StatusCode};
use models::transacao::{NewTransacao, TransacaoResponse};
use persistence::{PersistenceError, PostgresRepository};
use std::{env, sync::Arc};

mod models;
mod persistence;

type AppState = Arc<PostgresRepository>;

#[tokio::main]
async fn main() {

    let database_url = env::var("DATABASE_URL").unwrap();

    let repository = PostgresRepository::connect(&database_url, 30).await.unwrap();

    let app_state = Arc::new(repository);

    let app : Router = Router::new()
        .route("/clientes/:id_cliente/transacoes", post(create_transacao))
        .route("/clientes/:id_cliente/extrato", get(get_cliente))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

async fn get_cliente(
    State(repo): State<AppState>,
    Path(id_cliente): Path<i32>
    ) -> impl IntoResponse {

    match repo.find_cliente_by_id(id_cliente).await {
        Ok(Some(cliente)) => Ok(Json(cliente)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

async fn create_transacao(
        State(repo): State<AppState>,
        Path(id_cliente): Path<i32>,
        Json(new_transacao) : Json<NewTransacao>
    ) -> impl IntoResponse {

        match repo.create_transacao(new_transacao, id_cliente).await {
            Ok(transacao) => {
                Ok(Json(transacao))
            },
            Err(PersistenceError::NotEnoughFunds) => {
                Err(StatusCode::UNPROCESSABLE_ENTITY)
            },
            Err(PersistenceError::IdDoesNotExist) => {
                Err(StatusCode::NOT_FOUND)
            },
            _ => {
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }

}
