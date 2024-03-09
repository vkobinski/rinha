use axum::{Router, routing::{get, post}, response::IntoResponse, Json, extract::{Path, State}, http::StatusCode};
use models::transacao::NewTransacao;
use persistence::{PersistenceError, PostgresRepository};
use std::{env, sync::Arc};
use dotenv::dotenv;

mod models;
mod persistence;

type AppState = Arc<PostgresRepository>;

#[tokio::main]
async fn main() {

    dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap();
    let port = env::var("PROD_PORT").unwrap_or("9999".to_string());
    let max_connections = env::var("CONNECTIONS").unwrap_or(50.to_string());

    println!("connections: {}", max_connections);

    let repository = PostgresRepository::connect(&database_url, max_connections.parse::<u32>().unwrap()).await.unwrap();

    let app_state = Arc::new(repository);

    let app : Router = Router::new()
        .route("/clientes/:id_cliente/transacoes", post(create_transacao))
        .route("/clientes/:id_cliente/extrato", get(get_cliente))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();


}

async fn get_cliente(
    State(repo): State<AppState>,
    Path(id_cliente): Path<i32>
    ) -> impl IntoResponse {



    match repo.find_cliente_by_id(id_cliente).await {
        Ok(Some(cliente)) => Ok(Json(cliente)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(PersistenceError::IdDoesNotExist) => Err(StatusCode::NOT_FOUND),
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
