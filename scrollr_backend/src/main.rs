use std::{env, fs::{self}, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{Json, Router, debug_handler, extract::State, routing::{get, post}};
use axum_server::tls_rustls::RustlsConfig;
use finance_service::{start_finance_services, types::FinanceState, update_all_previous_closes};
use futures_util::future::join_all;
use dotenv::dotenv;
use scrollr_backend::SchedulePayload;
use sports_service::{frequent_poll, start_sports_service};
use utils::{database::{PgPool, initialize_pool, sports::LeagueConfigs}, log::{info, init_async_logger, warn}};

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

    let config = RustlsConfig::from_pem_file(
        PathBuf::from(env::var("CERT_PATH").expect("Cert path needs to be included in .env!")),
        PathBuf::from(env::var("KEY_PATH").expect("Key path needs to be included in .env!"))
    ).await.expect("Failed to set TLS config");

    let app = Router::new()
        .route("/", post(handler))
        .route("/finance", get(|| async { "Hello, World!" }))
        .with_state(database_pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();

    join_all(handles).await;

    println!("Closing...")
}

#[debug_handler]
async fn handler(State(db_pool): State<PgPool>, Json(payload): Json<SchedulePayload>) {
    let pool = Arc::new(db_pool);
    match payload.schedule_type.as_str() {
        "finance" => {
            let state = FinanceState::new(pool);

            info!("Running daily finance job...");
            update_all_previous_closes(state).await;
            info!("Previous closes updated!");
        }

        "sports" => {
            info!("Starting frequent polling for the following leagues {:?}", payload.data);
            let mut leagues = Vec::new();

            let file_contents = fs::read_to_string("./configs/leagues.json").unwrap();
            let leagues_to_ingest: Vec<LeagueConfigs> = serde_json::from_str(&file_contents).unwrap();

            for league in leagues_to_ingest {
                if payload.data.contains(&league.name) {
                    leagues.push(league);
                }
            }

            frequent_poll(leagues, &pool).await;
        }
        _ => warn!("Unexpected POST payload {}", payload.schedule_type),
    }
}