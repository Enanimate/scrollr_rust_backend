pub use oauth2::{http::header, reqwest::Client};
use utils::log::error;

use crate::{xml_types::FantasyContent, types::UserLeague};

const YAHOO_BASE_API: &str = "https://fantasysports.yahooapis.com/fantasy/v2";

async fn make_request(endpoint: &str, client: Client, token: &str) -> Option<String> {
    let response = client.get(format!("{YAHOO_BASE_API}{endpoint}"))
        .bearer_auth(token)
        .header(header::ACCEPT, "application/xml")
        .send()
        .await
        .inspect_err(|e| error!("Reqwest Error: {e}"));

    if let Ok(data) = response {
        return data.text().await.ok();
    } else {
        return None;
    }
}

pub async fn get_user_leagues(client: Client, token: &str, game_key: &str) -> Vec<UserLeague> {
    let response = make_request(&format!("/users;use_login=1/games;game_keys={game_key}/leagues"), client, token).await;
    if response.is_none() { return Vec::new() };

    let league_data = response.unwrap();

    if league_data.contains("token_rejected") {
        panic!("PLEASE HANDLE TOKEN REFRESH");
    }

    let cleaned: FantasyContent = serde_xml_rs::from_str(&league_data).unwrap();

    let mut leagues = Vec::new();

    let users = cleaned.users.user;
    let games = users[0].games.game.clone();
    let leagues_data = games[0].leagues.league.clone();

    for league in leagues_data {
        leagues.push(UserLeague {
            league_key: league.league_key,
            league_id: league.league_id,
            name: league.name,
            url: league.url,
            logo_url: league.logo_url,
            draft_status: league.draft_status,
            num_teams: league.num_teams,
            scoring_type: league.scoring_type,
            league_type: league.league_type,
            current_week: league.current_week,
            start_week: league.start_week,
            end_week: league.end_week,
            season: league.season,
            game_code: league.game_code,
        });
    }
    
    return leagues;
}

