use std::{collections::HashMap, env, fs, pin::Pin, sync::Arc, time::{Duration, Instant}};

use reqwest::{Client, header::{HeaderMap, HeaderValue}};
use serde::{Deserialize, Serialize};
use tokio::time::Sleep;
use utils::database::PgPool;

#[derive(Debug, Deserialize)]
pub(crate) struct TradeUpdate {
    #[serde(rename = "type")]
    pub message_type: String,
    pub data: Vec<TradeData>
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct TradeData {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "p")]
    pub price: f64,
    #[serde(rename = "t")]
    pub timestamp: u64,
}

#[derive(Debug, Default)]
pub(crate) struct BatchStats {
    pub batches_processed: u64,
    pub total_updates_processed: u64,
    pub errors: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct QuoteResponse {
    #[serde(rename = "c")]
    pub current_price: f64,
    #[serde(rename = "d")]
    pub change: f64,
    #[serde(rename = "dp")]
    pub percent_change: f64,
    #[serde(rename = "pc")]
    pub previous_close: f64
}

pub(crate) struct WebSocketState {
    pub update_queue: HashMap<String, TradeData>,
    pub batch_timer: Option<Pin<Box<Sleep>>>,
    pub is_processing_batch: bool,
    pub stats: BatchStats,
    pub last_log_time: Option<Instant>,
}

impl WebSocketState {
    pub fn new() -> Self {
        Self {
            update_queue: HashMap::new(),
            batch_timer: None,
            is_processing_batch: false,
            stats: BatchStats::default(),
            last_log_time: None,
        }
    }
}

#[derive(Clone)]
pub struct FinanceState {
    pub api_key: String,
    pub subscriptions: Vec<String>,
    pub client: Arc<Client>,
    pub pool: Arc<PgPool>,
}

impl FinanceState {
    pub fn new(pool: Arc<PgPool>) -> Self {
        let file_contents = fs::read_to_string("./configs/subscriptions.json").unwrap();
        let subscriptions = serde_json::from_str(&file_contents).unwrap();

        let api_key = env::var("FINNHUB_API_KEY").unwrap();

        let mut headers: HeaderMap = HeaderMap::new();
        headers.append("X-Finnhub-Token", HeaderValue::from_str(&api_key).unwrap());

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_millis(10_000))
            .build().unwrap();

        Self {
            api_key,
            subscriptions,
            client: Arc::new(client),
            pool,
        }
    }
}

#[derive(Serialize)]
pub struct FinanceHealth {
    pub status: String,
    pub batch_number: u64,
}

impl FinanceHealth {
    pub fn new() -> Self {
        Self {
            status: String::from("healthy"),
            batch_number: 0,
        }
    }

    pub(crate) fn set_batch(&mut self, number: u64) {
        self.batch_number = number
    }

    pub fn get_health(&self) -> Self {
        Self {
            status: self.status.clone(),
            batch_number: self.batch_number,
        }
    }
}