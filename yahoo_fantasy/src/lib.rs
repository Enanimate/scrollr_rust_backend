use std::{error::Error, sync::Arc};

use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl, basic::BasicClient, reqwest::Client};
use utils::database::{PgPool, fantasy::{create_tables, insert_csrf}};

use crate::types::Tokens;


const AUTH_URL: &str = "https://api.login.yahoo.com/oauth2/request_auth";
const TOKEN_URL: &str = "https://api.login.yahoo.com/oauth2/get_token";

pub mod api;
mod xml_leagues;
mod xml_standings;
mod xml_roster;
mod error;
pub mod stats;
pub mod types;

pub async fn start_fantasy_service(pool: Arc<PgPool>) {
    create_tables(pool).await;
}

pub async fn yahoo(pool: Arc<PgPool>, client_id: String, client_secret: String, callback_url: String) -> Result<(String, String), Box<dyn Error>> {
    let csrf_token = CsrfToken::new_random();

    insert_csrf(pool, csrf_token.clone().into_secret()).await;

    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(callback_url)?);

    //TODO: use csrf_token to validate
    let (auth_url, csrf_token) = client
        .authorize_url(|| csrf_token)
        .add_scope(Scope::new("fspt-r".to_string()))
        .url();
    
    return Ok((auth_url.as_str().to_string(), csrf_token.into_secret()));
}

pub async fn exchange_for_token(_pool: Arc<PgPool>, authorization_code: String, client_id: String, client_secret: String, _csrf: String, callback_url: String) -> Tokens {
    let client = BasicClient::new(ClientId::new(client_id.clone()))
        .set_client_secret(ClientSecret::new(client_secret.clone()))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string()).unwrap())
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(callback_url.clone()).unwrap());

    let http_client = Client::new();

    let token_result = client
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(&http_client)
        .await
        .unwrap();

    let access_token = token_result.access_token();
    let refresh_token = if let Some(token) = token_result.refresh_token() {
        Some(token.clone().into_secret())
    } else {
        None
    };

    return Tokens {
        access_token: access_token.clone().into_secret(),
        refresh_token: refresh_token,
        client_id,
        client_secret,
        callback_url,
        access_type: String::new(),
    };
}

pub(crate) async fn exchange_refresh(client_id: String, client_secret: String, callback_url: String, old_refresh_token: String) -> Result<(String, String), Box<dyn Error>> {
    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string())?)
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string())?)
        .set_redirect_uri(RedirectUrl::new(callback_url)?);

    let http_client = Client::new();
    let refresh_token = RefreshToken::new(old_refresh_token);

    let token_result = client
        .exchange_refresh_token(&refresh_token)
        .request_async(&http_client)
        .await?;

    let new_access_token = token_result.access_token().secret().to_string();

    let new_refresh_token = match token_result.refresh_token() {
        Some(t) => t.secret().to_string(),
        None => {
            refresh_token.secret().to_string()
        }
    };

    Ok((new_access_token, new_refresh_token))
}