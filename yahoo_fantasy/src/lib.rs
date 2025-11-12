use std::sync::Arc;

use oauth2::{AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl, basic::BasicClient, reqwest::Client};
use utils::database::{PgPool, fantasy::{create_tables, insert_csrf, update_tokens}};

const AUTH_URL: &str = "https://api.login.yahoo.com/oauth2/request_auth";
const TOKEN_URL: &str = "https://api.login.yahoo.com/oauth2/get_token";

pub async fn start_fantasy_service(pool: Arc<PgPool>) {
    create_tables(pool).await;
}

pub async fn yahoo(pool: Arc<PgPool>, client_id: String, client_secret: String, callback_url: String) -> (String, String) {
    let csrf_token = CsrfToken::new_random();

    insert_csrf(pool, csrf_token.clone().into_secret()).await;

    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string()).unwrap())
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(callback_url).unwrap());

    //TODO: use csrf_token to validate
    let (auth_url, csrf_token) = client
        .authorize_url(|| csrf_token)
        .add_scope(Scope::new("fspt-r".to_string()))
        .url();


    
    return (auth_url.as_str().to_string(), csrf_token.into_secret());
}

pub async fn exchange_for_token(pool: Arc<PgPool>, authorization_code: String, client_id: String, client_secret: String, csrf: String, callback_url: String) -> String {
    let client = BasicClient::new(ClientId::new(client_id))
        .set_client_secret(ClientSecret::new(client_secret))
        .set_auth_uri(AuthUrl::new(AUTH_URL.to_string()).unwrap())
        .set_token_uri(TokenUrl::new(TOKEN_URL.to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(callback_url).unwrap());

    let http_client = Client::new();

    let token_result = client
        .exchange_code(AuthorizationCode::new(authorization_code))
        .request_async(&http_client)
        .await
        .unwrap();

    let access_token = token_result.access_token();
    let refresh_token = token_result.refresh_token().unwrap();

    update_tokens(pool, csrf, access_token.clone(), refresh_token.clone()).await;

    return access_token.clone().into_secret();
}