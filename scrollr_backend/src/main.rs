use std::{env, fs::{self}, net::{IpAddr, Ipv4Addr, SocketAddr}, path::PathBuf, sync::Arc};

use axum::{Json, Router, extract::{Path, Query, State}, http::{HeaderMap, HeaderValue, StatusCode, header::{self, REFERRER_POLICY}}, response::{Html, IntoResponse, Redirect, Response}, routing::{get, post}};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};
use finance_service::{start_finance_services, types::FinanceState, update_all_previous_closes};
use futures_util::{StreamExt, future::join_all};
use dotenv::dotenv;
use scrollr_backend::{ErrorCodeResponse, RefreshBody, SchedulePayload, ServerState, get_access_token, update_tokens};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sports_service::{frequent_poll, start_sports_service};
use tokio_rustls_acme::{AcmeConfig, caches::DirCache, tokio_rustls::rustls::ServerConfig};
use tower_http::set_header::SetRequestHeaderLayer;
use utils::{database::sports::LeagueConfigs, log::{error, info, init_async_logger, warn}};
use yahoo_fantasy::{api::{get_league_standings, get_team_roster, get_user_leagues}, exchange_for_token, stats::{BasketballStats, FootballStats, HockeyStats, StatDecode}, types::{LeagueStandings, Roster, Tokens}, yahoo};

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
        .route("/finance/health", get(finance_health))
        .route("/yahoo/start", get(get_yahoo_handler))
        .route("/yahoo/callback", get(yahoo_callback))
        .route("/yahoo/leagues", get(user_leagues))
        .route("/yahoo/league/{league_key}/standings", get(league_standings))
        .route("/yahoo/team/{teamKey}/roster", get(team_roster))
        .route("/health", get(|| async { "Hello, World!" }))
        .layer(
            SetRequestHeaderLayer::if_not_present(
                header::HeaderName::from_static("x-frame-options"),
                HeaderValue::from_static("DENY")
            )
        )
        .with_state(web_state);

    let ipv4_addr = Ipv4Addr::from([0, 0, 0, 0]);

    let addr = SocketAddr::new(IpAddr::V4(ipv4_addr), 8443);

    info!("Listening on address: {addr}");

    axum_server::bind(addr)
        .acceptor(acceptor)
        .serve(app.into_make_service())
        .await
        .expect("Failed to bind to port");

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

async fn get_yahoo_handler(State(mut web_state): State<ServerState>) -> Response {
    info!("start!");
    let result = yahoo(web_state.client_id, web_state.client_secret, web_state.yahoo_callback.clone())
        .await
        .inspect_err(|e| error!("Yahoo Error: {e}"));

    let (redirect_url, csref_token) = result.unwrap();
    web_state.csref_token = Some(csref_token);

    let mut response = Redirect::temporary(&redirect_url).into_response();

    response.headers_mut().insert(
        REFERRER_POLICY, 
        HeaderValue::from_static("no-referrer")
    );

    response
}

#[derive(Deserialize)]
struct CodeResponse {
    code: String,
    state: String,
}

async fn yahoo_callback(Query(tokens): Query<CodeResponse>, State(web_state): State<ServerState>, jar: CookieJar) -> Response {
    info!("{}", web_state.yahoo_callback);
    let tokens_option = exchange_for_token(tokens.code, web_state.client_id, web_state.client_secret, tokens.state, web_state.yahoo_callback).await;
    if tokens_option.is_none() { return ErrorCodeResponse::new(StatusCode::INTERNAL_SERVER_ERROR, "Failed to retrieve tokens"); }

    let tokens = tokens_option.unwrap();
    let access_token = tokens.access_token;
    let refresh_token = if let Some(token) = tokens.refresh_token {
        token
    } else {
        String::new()
    };

    let cookie_auth = Cookie::build(("yahoo-auth", access_token.clone()))
        .path("/yahoo")
        .secure(true)
        .http_only(true) 
        .same_site(SameSite::Lax)
        .build();

    let cookie_refresh = Cookie::build(("yahoo-refresh", refresh_token.clone()))
        .path("/yahoo")
        .secure(true)
        .http_only(true)
        .same_site(SameSite::Lax)
        .build();

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
                                accessToken: {0},
                                refreshToken: {1}
                            }}, '*'); 
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
        serde_json::to_string(&access_token).unwrap_or_else(|_| "\"error\"".to_string()),
        serde_json::to_string(&refresh_token).unwrap_or_else(|_| "\"error\"".to_string()),
    );
    let cookies = jar.add(cookie_auth).add(cookie_refresh);

    (cookies, Html(html_content)).into_response()
}

async fn user_leagues(jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap, refresh_token: Option<Json<RefreshBody>>) -> Response {
    info!("start leagues!");
    let token_option = get_access_token(jar.clone(), headers, web_state.clone(), refresh_token);

    if token_option.is_none() { return ErrorCodeResponse::new(StatusCode::UNAUTHORIZED, "Unauthorized, missing access_token"); }

    let initial_tokens = token_option.unwrap();

    let response = get_user_leagues(&initial_tokens, web_state.client, "nfl").await;

    if let Err(e) = response {
        error!("Error fetching leagues for user: {e}");
        return ErrorCodeResponse::new(StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch leagues: {e}").as_str());
    }

    let (leagues, new_tokens) = response.unwrap();
    let mut headers = HeaderMap::new();
    let updated_cookies = update_tokens(&mut headers, jar, new_tokens, &initial_tokens.access_type);


    info!("end leagues");

    (headers, updated_cookies, Json(leagues)).into_response()
}

async fn league_standings(Path(league_key): Path<String>, jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap, refresh_token: Option<Json<RefreshBody>>) -> Response {
    info!("start standings!");
    let token_option = get_access_token(jar.clone(), headers, web_state.clone(), refresh_token);
    if token_option.is_none() { return ErrorCodeResponse::new(StatusCode::UNAUTHORIZED, "Unauthorized, missing access_token"); }

    let initial_tokens = token_option.unwrap();

    let response = get_league_standings(&league_key, web_state.client, &initial_tokens).await;

    if let Err(e) = response {
        error!("Error fetching standings for {league_key}: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let (standings, new_tokens) = response.unwrap();
    let mut headers = HeaderMap::new();
    let updated_cookies = update_tokens(&mut headers, jar, new_tokens, &initial_tokens.access_type);

    #[derive(Serialize)]
    struct Standings {
        standings: Vec<LeagueStandings>,
    }

    info!("end standings!");

    (headers, updated_cookies, Json(Standings { standings })).into_response()
}

#[derive(Deserialize)]
struct RosterQuery {
    date: Option<String>,
    sport: String,
}

async fn team_roster(Query(query): Query<RosterQuery>, Path(team_key): Path<String>, jar: CookieJar, State(web_state): State<ServerState>, headers: HeaderMap, refresh_token: Option<Json<RefreshBody>>) -> Response {
    info!("start roster! {team_key} {:?}", query.date);
    let token_option = get_access_token(jar.clone(), headers, web_state.clone(), refresh_token);
    if token_option.is_none() { return ErrorCodeResponse::new(StatusCode::UNAUTHORIZED, "Unauthorized, missing access_token"); }

    let initial_tokens = token_option.unwrap();

    fn create_response<T>(roster_vec: Vec<Roster<T>>, jar: CookieJar, new_tokens: Option<(String, String)>, inital_tokens: Tokens) -> Response 
    where 
        T: StatDecode + std::fmt::Display + serde::Serialize,
        <T as TryFrom<u8>>::Error: std::fmt::Display
    {
        let mut headers = HeaderMap::new();
        let updated_cookies = update_tokens(&mut headers, jar, new_tokens, &inital_tokens.access_type);

        let response_json = json!({
            "roster": roster_vec,
        });

        (headers, updated_cookies, Json(response_json)).into_response()
    }

    let result = match query.sport.as_str() {
        "nfl" | "football" => {
            let response = get_team_roster::<FootballStats>(&team_key, web_state.client, &initial_tokens, query.date.clone()).await;
            match response {
                Ok((roster, new_tokens)) => Ok(create_response(roster, jar, new_tokens, initial_tokens)),
                Err(e) => Err(e)
            }
        }

        "nba" | "basketball" => {
            let response = get_team_roster::<BasketballStats>(&team_key, web_state.client, &initial_tokens, query.date.clone()).await;
            match response {
                Ok((roster, new_tokens)) => Ok(create_response(roster, jar, new_tokens, initial_tokens)),
                Err(e) => Err(e)
            }
        }

        "nhl" | "hockey" => {
            let response = get_team_roster::<HockeyStats>(&team_key, web_state.client, &initial_tokens, query.date.clone()).await;
            match response {
                Ok((roster, new_tokens)) => Ok(create_response(roster, jar, new_tokens, initial_tokens)),
                Err(e) => Err(e)
            }
        }

        _ => {
            error!("Unsupported sport type: {}", query.sport);
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    if let Err(e) = result {
        error!("Error fetching roster for {team_key}: {e}");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    let final_response = result.unwrap();

    info!("end roster! {team_key} {:?}", query.date);
    final_response
}

async fn finance_health(State(web_state): State<ServerState>) -> impl IntoResponse {
    let health = web_state.finance_health.lock().await.get_health();

    Json(health)
}