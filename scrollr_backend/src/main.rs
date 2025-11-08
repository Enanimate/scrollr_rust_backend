use std::path::PathBuf;

use finance_service::start_finance_services;
use futures_util::future::join_all;
use dotenv::dotenv;
use utils::{info, init_async_logger};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let mut handles = Vec::new();

    match init_async_logger(PathBuf::from("websocket.log")) {
        Ok(_) => info!("Async logging initialized successfully"),
        Err(e) => eprintln!("Failed to set logger: {}", e)
    }

    handles.push(tokio::spawn(start_finance_services()));

    join_all(handles).await;

    println!("Closing...")
}