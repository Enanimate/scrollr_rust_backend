use std::{env, sync::Arc};

use log::error;
use magic_crypt::{MagicCryptTrait, new_magic_crypt};
use oauth2::{AccessToken, RefreshToken};
pub use sqlx::{PgPool, query, types::Uuid};

pub async fn create_tables(pool: Arc<PgPool>) {
    let statement = "
        CREATE TABLE IF NOT EXISTS internal.fantasy_userdata (
            user_id UUID PRIMARY KEY UNIQUE NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
            csrf TEXT DEFAULT NULL,
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

pub async fn insert_csrf(pool: Arc<PgPool>, csrf: String, user_id: Uuid) {
    let statement = "
        UPDATE internal.fantasy_userdata
        SET csrf = $1
        WHERE user_id = $2
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .bind(csrf)
            .bind(user_id)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}

pub async fn update_tokens(pool: Arc<PgPool>, access_token: AccessToken, refresh_token: RefreshToken, user_id: Uuid) {

    let key = env::var("ENCRYPTION_KEY").unwrap();
    let mc = new_magic_crypt!(&key, 256);

    let encrypted_access_token = mc.encrypt_str_to_base64(access_token.secret());
    let encrypted_refresh_token = mc.encrypt_str_to_base64(refresh_token.secret());

    let statement = "
        UPDATE internal.fantasy_userdata
        SET access_token = $1,
            refresh_token = $2
        WHERE user_id = $3
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .bind(encrypted_access_token)
            .bind(encrypted_refresh_token)
            .bind(user_id)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}