use finance_service::start_finance_services;
use futures_util::future::join_all;
use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let mut handles = Vec::new();

    handles.push(tokio::spawn(start_finance_services()));

    join_all(handles).await;

    println!("Closing...")
}