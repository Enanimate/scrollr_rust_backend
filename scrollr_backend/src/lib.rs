use std::{env, sync::Arc};

use finance_service::types::FinanceHealth;
use serde::Deserialize;
use tokio::sync::Mutex;
use utils::database::{PgPool, initialize_pool};
use yahoo_fantasy::api::Client;

#[derive(Debug, Deserialize)]
pub struct SchedulePayload {
    pub schedule_type: String,
    pub data: Vec<String>,
}

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Arc<PgPool>,
    pub client_id: String,
    pub client_secret: String,
    pub yahoo_callback: String,
    pub csref_token: Option<String>,
    pub client: Client,

    pub finance_health: Arc<Mutex<FinanceHealth>>,
}

impl ServerState {
    pub async fn new() -> Self {
        Self {
            db_pool: Arc::new(initialize_pool().await.expect("Failed to initialize database pool")),
            client_id: env::var("YAHOO_CLIENT_ID").expect("Yahoo client ID must be set in .env"),
            client_secret: env::var("YAHOO_CLIENT_SECRET").expect("Yahoo client secret must be set in .env"),
            yahoo_callback: format!("https://{}{}", env::var("DOMAIN_NAME").unwrap(), env::var("YAHOO_CALLBACK_URL").expect("Yahoo callback URL must be set in .env")),
            csref_token: None,
            client: Client::new(),

            finance_health: Arc::new(Mutex::new(FinanceHealth::new())),
        }
    }
}