use std::{env, sync::Arc};

use axum::{Json, http::{HeaderMap, HeaderValue, StatusCode, header::AUTHORIZATION}, response::{IntoResponse, Response}};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};
use finance_service::types::FinanceHealth;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use utils::{database::{PgPool, initialize_pool}, log::warn};
use yahoo_fantasy::{api::Client, types::Tokens};

#[derive(Serialize)]
pub struct ErrorCodeResponse {
    status: String,
    message: String,
}

impl ErrorCodeResponse {
    pub fn new(status: StatusCode, message: &str) -> Response {
        let response = ErrorCodeResponse {
            status: status.as_str().to_string(),
            message: message.to_string()
        };

        (status, Json(response)).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct SchedulePayload {
    pub schedule_type: String,
    pub data: Vec<String>,
}

#[derive(Clone)]
pub struct ServerState {
    pub db_pool: Arc<PgPool>,
    pub client_id: String,
    pub client_secret: String,
    pub yahoo_callback: String,
    pub csref_token: Option<String>,
    pub client: Client,

    pub finance_health: Arc<Mutex<FinanceHealth>>,
}

impl ServerState {
    pub async fn new() -> Self {
        Self {
            db_pool: Arc::new(initialize_pool().await.expect("Failed to initialize database pool")),
            client_id: env::var("YAHOO_CLIENT_ID").expect("Yahoo client ID must be set in .env"),
            client_secret: env::var("YAHOO_CLIENT_SECRET").expect("Yahoo client secret must be set in .env"),
            yahoo_callback: format!("https://{}{}", env::var("DOMAIN_NAME").unwrap(), env::var("YAHOO_CALLBACK_URL").expect("Yahoo callback URL must be set in .env")),
            csref_token: None,
            client: Client::new(),

            finance_health: Arc::new(Mutex::new(FinanceHealth::new())),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RefreshBody {
    refresh_token: String
}

pub fn get_access_token(jar: CookieJar, headers: HeaderMap, web_state: ServerState, refresh_token: Option<Json<RefreshBody>>) -> Option<Tokens> {
    if let Some(auth_token) = headers.get(AUTHORIZATION) {
        //let refresh_token = headers.get("refresh_token");
        let access_token = auth_token
            .to_str()
            .inspect_err(|e| warn!("Access Token could not be cast as str: {e}"));

        if let Ok(token) = access_token {
            let fixed_token = if token.starts_with("Bearer ") {
                token.strip_prefix("Bearer ").unwrap()
            } else {
                token
            };

            let refresh = if let Some(token) = refresh_token {
                Some(token.refresh_token.clone())
            } else {
                None
            };

            return Some(Tokens {
                access_token: fixed_token.to_string(),
                refresh_token: refresh,
                client_id: web_state.client_id,
                client_secret: web_state.client_secret,
                callback_url: web_state.yahoo_callback,
                access_type: String::from("header")
            });
        } else {
            return None;
        }
    } else {
        if let Some(auth_cookie) = jar.get("yahoo-auth") {
            let token = auth_cookie.value_trimmed();
            let refresh_cookie = jar.get("yahoo-refresh");

            let refresh = if let Some(token) = refresh_cookie {
                Some(token.value_trimmed().to_string())
            } else {
                None
            };

            return Some(Tokens {
                access_token: token.to_string(),
                refresh_token: refresh,
                client_id: web_state.client_id,
                client_secret: web_state.client_secret,
                callback_url: web_state.yahoo_callback,
                access_type: String::from("cookie")
            });
        } else {
            return None;
        }
    }
}

pub fn update_tokens(headers: &mut HeaderMap, jar: CookieJar, new_tokens: Option<(String, String)>, access_type: &str) -> CookieJar {
    if let Some((access_token, refresh_token)) = new_tokens {
        if access_type == "cookie" {
            let cookie_auth = Cookie::build(("yahoo-auth", access_token))
            .path("/yahoo")
            .secure(true)
            .http_only(true) 
            .same_site(SameSite::Lax)
            .build();

        let cookie_refresh = Cookie::build(("yahoo-refresh", refresh_token))
            .path("/yahoo")
            .secure(true)
            .http_only(true)
            .same_site(SameSite::Lax)
            .build();

        return jar.add(cookie_auth).add(cookie_refresh);
        } else {
            let access_result = HeaderValue::from_str(&access_token);
            let refresh_result = HeaderValue::from_str(&refresh_token);

            if let Ok(access_header) = access_result {
                headers.insert("X-New-Access-Token", access_header);
            }
            if let Ok(refresh_header) = refresh_result {
                headers.insert("X-New-Refresh-Token", refresh_header);
            }
        }
    }

    return jar;
}