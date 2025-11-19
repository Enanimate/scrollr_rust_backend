pub use oauth2::{http::header, reqwest::Client};
use utils::log::error;

use crate::{types::{LeagueStandings, Roster, UserLeague}, xml_leagues, xml_roster, xml_standings};

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

    let cleaned: xml_leagues::FantasyContent = serde_xml_rs::from_str(&league_data).unwrap();

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

pub async fn get_league_standings(league_key: String, client: Client, token: String) -> Vec<LeagueStandings> {
    let response = make_request(&format!("/league/{league_key}/standings"), client, &token).await;
    if response.is_none() { return Vec::new() };

    let league_data = response.unwrap();

    if league_data.contains("token_rejected") {
        panic!("PLEASE HANDLE TOKEN REFRESH");
    }

    let cleaned: xml_standings::FantasyContent = serde_xml_rs::from_str(&league_data).unwrap();

    let mut standings = Vec::new();

    let league = cleaned.league;
    let teams = league.standings.teams.team;
    
    for team in teams {
        let team_standings = team.team_standings;
        let outcome_total = team_standings.outcome_totals;

        let percentage = outcome_total.percentage.unwrap_or_else(|| "0.0".to_string());
        let games_back = team_standings.games_back.unwrap_or(0.0);
        standings.push(
            LeagueStandings {
                team_key: team.team_key,
                team_id: team.team_id,
                name: team.name,
                url: team.url,
                team_logo: team.team_logos.team_logo[0].url.clone(),
                wins: outcome_total.wins,
                losses: outcome_total.losses,
                ties: outcome_total.ties,
                percentage: percentage,
                games_back: games_back,
                points_for: team_standings.points_for,
                points_against: team_standings.points_against,
            }
        );
    }

    return standings;
}

pub async fn get_team_roster(team_key: String, client: Client, token: String, opt_date: Option<String>) -> Vec<Roster> {
    let url = if let Some(date) = opt_date {
        format!("/team/{team_key}/roster;date={date}/players/stats")
    } else {
        format!("/team/{team_key}/roster/players/stats")
    };

    let response = make_request(&url, client, &token).await;

    if response.is_none() { return Vec::new() };

    let league_data = response.unwrap();

    if league_data.contains("token_expired") {
        panic!("PLEASE HANDLE TOKEN REFRESH");
    }

    let cleaned: xml_roster::FantasyContent = serde_xml_rs::from_str(&league_data).unwrap();

    let mut roster = Vec::new();

    let team = cleaned.team;
    let players = team.roster.players.player;
    for player in players {
        let eligible = player.eligible_positions.position;
        let model = Roster {
            id: player.player_id,
            key: player.player_key,
            name: player.name.full,
            first_name: player.name.first,
            last_name: player.name.last,
            team_abbreviation: player.editorial_team_abbr,
            team_full_name: player.editorial_team_full_name,
            uniform_number: player.uniform_number.unwrap_or("None".to_string()),
            position: player.display_position,
            selected_position: player.selected_position.position,
            eligible_positions: eligible,
            image_url: player.image_url,
            headshot: player.headshot.url,
            is_undroppable: player.is_undroppable,
            position_type: player.position_type,
        };

        roster.push(model);
    }

    return roster;
}