use std::{env, fs::{self}, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{Json, Router, debug_handler, extract::State, routing::{get, post}};
use finance_service::{start_finance_services, types::FinanceState, update_all_previous_closes};
use futures_util::{StreamExt, future::join_all};
use dotenv::dotenv;
use scrollr_backend::SchedulePayload;
use sports_service::{frequent_poll, start_sports_service};
use tokio_rustls_acme::{AcmeConfig, caches::DirCache, tokio_rustls::rustls::ServerConfig};
use utils::{database::{PgPool, initialize_pool, sports::LeagueConfigs}, log::{error, info, init_async_logger, warn}};

#[tokio::main]
async fn main() {
    dotenv().ok();
    rustls::crypto::ring::default_provider().install_default().expect("Failed to install rustls crypto provider");
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    match init_async_logger("./logs") {
        Ok(_) => info!("Async logging initialized successfully"),
        Err(e) => eprintln!("Failed to set logger: {}", e)
    }

    let database_pool = initialize_pool().await.unwrap();
    let arc_pool = Arc::new(database_pool.clone());

    handles.push(tokio::spawn(start_finance_services(arc_pool.clone())));
    handles.push(tokio::spawn(start_sports_service(arc_pool)));

    let domain_name = env::var("DOMAIN_NAME").expect("Domain name needs to be specified in .env!");
    let contact_email = env::var("CONTACT_EMAIL").expect("Contact email needs to be specified in .env!");

    let cache_dir = PathBuf::from("./acme_cache");

    let mut state = AcmeConfig::new(vec![domain_name])
        .contact(vec![format!("mailto:{}", contact_email)])
        .cache_option(Some(cache_dir).map(DirCache::new))
        .directory_lets_encrypt(true)
        .state();

    let rustls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(state.resolver());
    let acceptor = state.axum_acceptor(Arc::new(rustls_config));

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => info!("event: {ok:?}"),
                Err(err) => error!("error: {err:?}"),
            }
        }
    });

    let app = Router::new()
        .route("/", post(handler))
        .route("/finance", get(|| async { "Hello, World!" }))
        .with_state(database_pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));

    axum_server::bind(addr)
        .acceptor(acceptor)
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