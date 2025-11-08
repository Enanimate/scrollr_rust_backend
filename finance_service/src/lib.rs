use std::sync::Arc;

use utils::database::PgPool;

use crate::{types::FinanceState, websocket::connect};

mod types;
mod websocket;

pub async fn start_finance_services(pool: Arc<PgPool>) {
    let state = FinanceState::new();

    connect(state.subscriptions, state.api_key, pool).await;
}