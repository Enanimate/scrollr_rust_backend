use std::sync::Arc;

use log::error;
use sqlx::{PgPool, query};

pub async fn create_tables(pool: Arc<PgPool>) {
    let statement = "
        CREATE TABLE IF NOT EXISTS trades (
            id SERIAL PRIMARY KEY,
            symbol VARCHAR(30) UNIQUE NOT NULL,
            price DECIMAL(10,2),
            previous_close DECIMAL(10,2),
            price_change DECIMAL(10,2),
            percentage_change DECIMAL(5,2),
            direction VARCHAR(10),
            last_updated TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
        );
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}