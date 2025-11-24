use anyhow::{Context, anyhow};
pub use oauth2::{http::header, reqwest::Client};
use utils::log::info;

use crate::{error::YahooError, stats::StatDecode, types::{LeagueStandings, Leagues, Roster, Tokens, UserLeague}, xml_leagues, xml_roster, xml_standings};

pub(crate) const YAHOO_BASE_API: &str = "https://fantasysports.yahooapis.com/fantasy/v2";

pub(crate) async fn make_request(endpoint: &str, client: Client, tokens: &Tokens, mut retries_allowed: u8) -> anyhow::Result<(String, Option<(String, String)>)> {
    let mut new_tokens: Option<(String, String)> = None;

    while retries_allowed > 0 {
        let access_token = if let Some(ref token) = new_tokens {
            token.0.clone()
        } else {
            tokens.access_token.clone()
        };

        let url = format!("{YAHOO_BASE_API}{endpoint}");
        let response = client.get(&url)
            .bearer_auth(access_token)
            .header(header::ACCEPT, "application/xml")
            .send()
            .await
            .with_context(|| format!("Failed to make request to {url}"))?
            .text()
            .await
            .with_context(|| format!("Failed casting response to text: {url}"))?;

        let status = YahooError::check_response(response.clone(), tokens.client_id.clone(), tokens.client_secret.clone(), tokens.callback_url.clone(), tokens.refresh_token.clone()).await;
        retries_allowed -= 1;
        match status {
            YahooError::Ok => return Ok((response, new_tokens)),
            YahooError::NewTokens(a, b) => new_tokens = Some((a, b)),
            YahooError::Failed => return Err(anyhow!("Request failed and could not be recovered")),
            YahooError::Error(e) => info!("{e}"),
        }
    }

    Err(anyhow!("Exceeded number of retries allowed"))
}

pub async fn get_user_leagues(tokens: &Tokens, client: Client, _game_key: &str) -> anyhow::Result<(Leagues, Option<(String, String)>)> {
    let (league_data, opt_tokens) = make_request(&format!("/users;use_login=1/games/leagues"), client, &tokens, 2).await?;

    let cleaned: xml_leagues::FantasyContent = serde_xml_rs::from_str(&league_data)?;

    let mut nba = Vec::new();
    let mut nfl = Vec::new();
    let mut nhl = Vec::new();

    let users = cleaned.users.user;
    let games = users[0].games.game.clone();

    for game in games {
        let league_data = game.leagues.league.clone();

        for league in league_data {
            let user_league = UserLeague {
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
            };

            match user_league.game_code.as_str() {
                "nba" => nba.push(user_league),
                "nfl" => nfl.push(user_league),
                "nhl" => nhl.push(user_league),
                _ => (),
            }
        }
    }

    let leagues = Leagues {
        nba,
        nfl,
        nhl,
    };
    
    return Ok((leagues, opt_tokens));
}

pub async fn get_league_standings(league_key: &str, client: Client, tokens: &Tokens) -> anyhow::Result<(Vec<LeagueStandings>, Option<(String, String)>)> {
    let (league_data, opt_tokens) = make_request(&format!("/league/{league_key}/standings"), client, &tokens, 2).await?;


    let cleaned: xml_standings::FantasyContent = serde_xml_rs::from_str(&league_data)?;

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

    return Ok((standings, opt_tokens));
}

pub async fn get_team_roster<T> (team_key: &str, client: Client, tokens: &Tokens, opt_date: Option<String>) -> anyhow::Result<(Vec<Roster<T>>, Option<(String, String)>)> 
where 
    T: StatDecode + serde::de::DeserializeOwned + std::fmt::Display,
    <T as TryFrom<u8>>::Error: std::fmt::Display,
{
    let url = if let Some(date) = opt_date {
        format!("/team/{team_key}/roster;date={date}/players/stats")
    } else {
        format!("/team/{team_key}/roster/players/stats")
    };

    let (league_data, opt_tokens) = make_request(&url, client, &tokens, 2).await?;

    let cleaned: xml_roster::FantasyContent<T> = serde_xml_rs::from_str(&league_data)?;

    let mut roster = Vec::new();

    let team = cleaned.team;
    let players = team.roster.players.player;
    for player in players.unwrap_or(Vec::new()) {
        let eligible = player.eligible_positions.position;
        let stats = player.player_stats.stats.stat;

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
            stats: stats,
            player_points: player.player_points,
        };

        roster.push(model);
    }

    return Ok((roster, opt_tokens));
}