use std::{env, sync::Arc};

use log::error;
use magic_crypt::{MagicCryptTrait, new_magic_crypt};
use oauth2::{AccessToken, RefreshToken};
use sqlx::{PgPool, query};

pub async fn create_tables(pool: Arc<PgPool>) {
    let statement = "
        CREATE TABLE IF NOT EXISTS fantasy_userdata (
            id SERIAL PRIMARY KEY,
            csrf TEXT UNIQUE NOT NULL,
            access_token TEXT DEFAULT NULL,
            refresh_token TEXT DEFAULT NULL,
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

pub async fn insert_csrf(pool: Arc<PgPool>, csrf: String) {
    let statement = "
        INSERT INTO fantasy_userdata (csrf)
            VALUES ($1)
        ON CONFLICT (csrf) DO NOTHING
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .bind(csrf)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}

pub async fn update_tokens(pool: Arc<PgPool>, csrf: String, access_token: AccessToken, refresh_token: RefreshToken) {

    let key = env::var("ENCRYPTION_KEY").unwrap();
    let mc = new_magic_crypt!(&key, 256);

    let encrypted_access_token = mc.encrypt_str_to_base64(access_token.secret());
    let encrypted_refresh_token = mc.encrypt_str_to_base64(refresh_token.secret());

    let statement = "
        UPDATE fantasy_userdata
        SET access_token = $1,
            refresh_token = $2
        WHERE csrf = $3
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .bind(encrypted_access_token)
            .bind(encrypted_refresh_token)
            .bind(csrf)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}