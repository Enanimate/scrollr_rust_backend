use std::sync::Arc;

use reqwest::Client;
use utils::database::PgPool;

use crate::{types::{FinanceState, QuoteResponse}, websocket::connect};

mod types;
mod websocket;

pub async fn start_finance_services(pool: Arc<PgPool>) {
    let state = FinanceState::new();

    connect(state.subscriptions, state.api_key, state.client, pool).await;
}

async fn get_quote(symbol: String, client: Arc<Client>) -> anyhow::Result<QuoteResponse> {
        let request = client.get(format!("https://finnhub.io/api/v1/quote?symbol={}", symbol)).build()?;

        let response = client.execute(request).await?.text().await?;
        let data: QuoteResponse = serde_json::from_str(&response)?;

        Ok(data)
}