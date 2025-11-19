use std::{env, fs::{self}, net::SocketAddr, path::PathBuf, sync::Arc};

use axum::{Json, Router, extract::{Path, Query, State}, http::{HeaderMap, StatusCode, header::AUTHORIZATION}, response::{Html, IntoResponse, Redirect, Response}, routing::{get, post}};
use axum_extra::extract::CookieJar;
use finance_service::{start_finance_services, types::FinanceState, update_all_previous_closes};
use futures_util::{StreamExt, future::join_all};
use dotenv::dotenv;
use scrollr_backend::{SchedulePayload, ServerState, get_access_token};
use serde::Deserialize;
use sports_service::{frequent_poll, start_sports_service};
use tokio_rustls_acme::{AcmeConfig, caches::DirCache, tokio_rustls::rustls::ServerConfig};
use utils::{database::sports::LeagueConfigs, log::{error, info, init_async_logger, warn}};
use yahoo_fantasy::{api::{get_league_standings, get_team_roster, get_user_leagues}, exchange_for_token, start_fantasy_service, yahoo};

#[tokio::main]
async fn main() {
    dotenv().ok();
    rustls::crypto::ring::default_provider().install_default().expect("Failed to install rustls crypto provider");
    let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    match init_async_logger("./logs") {
        Ok(_) => info!("Async logging initialized successfully"),
        Err(e) => eprintln!("Failed to set logger: {}", e)
    }

    let domain_name = env::var("DOMAIN_NAME").expect("Domain name needs to be specified in .env!");
    let contact_email = env::var("CONTACT_EMAIL").expect("Contact email needs to be specified in .env!");
    let web_state = ServerState::new().await;

    handles.push(tokio::spawn(start_finance_services(web_state.db_pool.clone(), Arc::clone(&web_state.finance_health))));
    handles.push(tokio::spawn(start_sports_service(web_state.db_pool.clone())));
    handles.push(tokio::spawn(start_fantasy_service(web_state.db_pool.clone())));

    let cache_dir = PathBuf::from("./acme_cache");

    let mut state = AcmeConfig::new(vec![domain_name])
        .contact(vec![format!("mailto:{}", contact_email)])
        .cache_option(Some(cache_dir).map(DirCache::new))
        .directory_lets_encrypt(true)
        .state();

    let rustls_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_cert_resolver(state.resolver());
    let acceptor = state.axum_acceptor(Arc::new(rustls_config));

    tokio::spawn(async move {
        loop {
            match state.next().await.unwrap() {
                Ok(ok) => info!("event: {ok:?}"),
                Err(err) => error!("error: {err:?}"),
            }
        }
    });

    let app = Router::new()
        .route("/", post(handler))
        .route("/yahoo/start", get(get_yahoo_handler))
        .route("/callback", get(yahoo_callback))
        .route("/yahoo/leagues", get(user_leagues))
        .route("/finance/health", get(finance_health))
        .route("/yahoo/league/{league_key}/standings", get(league_standings))
        .route("/yahoo/team/{teamKey}/roster", get(team_roster))
        .with_state(web_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8443));

    info!("Listening on address: {addr}");

    axum_server::bind(addr)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
        .unwrap();

    join_all(handles).await;

    println!("Closing...")
}

async fn handler(State(web_state): State<ServerState>, Json(payload): Json<SchedulePayload>) {
    let pool = web_state.db_pool;
    match payload.schedule_type.as_str() {
        "finance" => {
            let state = FinanceState::new(pool);

            info!("Running daily finance job...");
            update_all_previous_closes(state).await;
            info!("Previous closes updated!");
        }

        "sports" => {
            info!("Starting frequent polling for the following leagues {:?}", payload.data);
            let mut leagues = Vec::new();

            let file_contents = fs::read_to_string("./configs/leagues.json").unwrap();
            let leagues_to_ingest: Vec<LeagueConfigs> = serde_json::from_str(&file_contents).unwrap();

            for league in leagues_to_ingest {
                if payload.data.contains(&league.name) {
                    leagues.push(league);
                }
            }

            frequent_poll(leagues, &pool).await;
        }
        _ => warn!("Unexpected POST payload {}", payload.schedule_type),
    }
}

async fn get_yahoo_handler(State(mut web_state): State<ServerState>) -> Redirect {
    info!("Requested!");
    let result = yahoo(web_state.db_pool, web_state.client_id, web_state.client_secret, web_state.yahoo_callback.clone())
        .await
        .inspect_err(|e| error!("Yahoo Error: {e}"));

    let (redirect_url, csref_token) = result.unwrap();
    web_state.csref_token = Some(csref_token);

    Redirect::temporary(&redirect_url)
}

#[derive(Deserialize)]
struct CodeResponse {
    code: String,
    state: String,
}

async fn yahoo_callback(Query(tokens): Query<CodeResponse>, State(web_state): State<ServerState>) -> Result<Html<String>, StatusCode> {
    let access_token = exchange_for_token(web_state.db_pool, tokens.code, web_state.client_id, web_state.client_secret, tokens.state, web_state.yahoo_callback).await;

    let html_content = format!(
        r#"
            <!doctype html><html><head><meta charset="utf-8"><title>Auth Complete</title></head>
            <body style="font-family: ui-sans-serif, system-ui;">
                <script>
                (function() {{
                    try {{
                        if (window.opener) {{
                            // POST MESSAGE: Sending the access token back to the main app window
                            window.opener.postMessage({{ 
                                type: 'yahoo-auth', 
                                accessToken: {0} 
                            }}, '*'); 
                        }} else {{
                            document.cookie = `yahoo-auth={0}; domain=enanimate.dev`;
                        }}
                    }} catch(e) {{
                        console.error("Error sending token via postMessage:", e);
                    }}
                    // Always close the popup window after a brief delay
                    setTimeout(function(){{ window.close(); }}, 100);
                }})();
                </script>
                <p>Authentication successful. You can close this window.</p>
            </body></html>
        "#,
        serde_json::to_string(&access_token).unwrap_or_else(|_| "\"error\"".to_string())
    );

    Ok(Html(html_content))
}

async fn user_leagues(jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap) -> Response {
    if let Some(auth_token) = headers.get(AUTHORIZATION) {
        let access_token = auth_token
            .to_str()
            .inspect_err(|e| warn!("Access Token could not be cast as str: {e}"));

        if let Ok(token) = access_token {
            let fixed_token = if token.starts_with("Bearer ") {
                token.strip_prefix("Bearer ").unwrap()
            } else {
                token
            };

            let data = get_user_leagues(web_state.client, fixed_token, "nfl").await;

            return Json(data).into_response();
        } else {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    } else {
        if let Some(auth_cookie) = jar.get("yahoo-auth") {
            let token = auth_cookie.value_trimmed();

            let data = get_user_leagues(web_state.client, token, "nfl").await;
            return Json(data).into_response();
        } else {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }
}

async fn league_standings(Path(league_key): Path<String>, jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap) -> Response {
    let token_option = get_access_token(jar, headers);
    if token_option.is_none() { return StatusCode::UNAUTHORIZED.into_response() }

    let token = token_option.unwrap();

    let standings = get_league_standings(league_key, web_state.client, token).await;

    Json(standings).into_response()
}

async fn team_roster(Path(team_key): Path<String>, jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap) -> Response {
    let token_option = get_access_token(jar, headers);
    if token_option.is_none() { return StatusCode::UNAUTHORIZED.into_response() }

    let token = token_option.unwrap();

    let roster = get_team_roster(team_key, web_state.client, token, None).await;

    Json(roster).into_response()
}

async fn finance_health(State(web_state): State<ServerState>) -> Response {
    let health = web_state.finance_health.lock().await.get_health();

    Json(health).into_response()
}