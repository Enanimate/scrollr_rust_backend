use std::sync::Arc;

use axum::{Json, Router, debug_handler, extract::State, routing::{get, post}};
use finance_service::{start_finance_services, types::FinanceState, update_all_previous_closes};
use futures_util::future::join_all;
use dotenv::dotenv;
use scrollr_backend::SchedulePayload;
use sports_service::start_sports_service;
use utils::{database::{PgPool, initialize_pool}, log::{info, init_async_logger, warn}};

#[tokio::main]
async fn main() {
    dotenv().ok();
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    match init_async_logger("./logs") {
        Ok(_) => info!("Async logging initialized successfully"),
        Err(e) => eprintln!("Failed to set logger: {}", e)
    }

    let database_pool = initialize_pool().await.unwrap();
    let arc_pool = Arc::new(database_pool.clone());

    handles.push(tokio::spawn(start_finance_services(arc_pool.clone())));
    handles.push(tokio::spawn(start_sports_service(arc_pool)));

    let app = Router::new()
        .route("/", post(handler))
        .route("/finance", get(|| async { "Hello, World!" }))
        .with_state(database_pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    join_all(handles).await;

    println!("Closing...")
}

#[debug_handler]
async fn handler(State(db_pool): State<PgPool>, Json(payload): Json<SchedulePayload>) {
    match payload.schedule_type.as_str() {
        "finance" => {
            let pool = Arc::new(db_pool);
            let state = FinanceState::new(pool);

            info!("Running daily finance job...");
            update_all_previous_closes(state).await;
            info!("Previous closes updated!");
        }
        _ => warn!("Unexpected POST payload {}", payload.schedule_type),
    }
}