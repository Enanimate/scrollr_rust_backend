use std::{env, sync::Arc};

use serde::Deserialize;
use utils::database::{PgPool, initialize_pool};

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
}

impl ServerState {
    pub async fn new() -> Self {
        Self {
            db_pool: Arc::new(initialize_pool().await.expect("Failed to initialize database pool")),
            client_id: env::var("YAHOO_CLIENT_ID").expect("Yahoo client ID must be set in .env"),
            client_secret: env::var("YAHOO_CLIENT_SECRET").expect("Yahoo client secret must be set in .env"),
            yahoo_callback: env::var("YAHOO_CALLBACK_URL").expect("Yahoo callback URL must be set in .env"),
            csref_token: None
        }
    }
}