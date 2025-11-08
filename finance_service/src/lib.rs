use crate::{types::FinanceState, websocket::connect};

mod types;
mod websocket;

pub async fn start_finance_services() {
    let state = FinanceState::new();

    connect(state.subscriptions, state.api_key).await;
}