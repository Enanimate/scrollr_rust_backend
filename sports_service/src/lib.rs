use std::{fs, sync::Arc};
use chrono::NaiveDateTime;
use reqwest::Client;
use utils::{database::{PgPool, sports::{CleanedData, Team, clear_tables, create_tables, upsert_game}}, log::info};

use utils::database::sports::LeagueConfigs;

use crate::types::ScoreboardResponse;

mod types;

pub async fn start_sports_service(pool: Arc<PgPool>) {
    info!("Starting sports service...");

    info!("Creating sports tables...");
    create_tables(pool.clone()).await;

    let file_contents = fs::read_to_string("./leagues.json").unwrap();
    let leagues_to_ingest: Vec<LeagueConfigs> = serde_json::from_str(&file_contents).unwrap();

    info!("Beginning league ingest");
    ingest_data(leagues_to_ingest, pool).await;
}

async fn ingest_data(leagues: Vec<LeagueConfigs>, pool: Arc<PgPool>) {
    clear_tables(pool.clone(), leagues.clone()).await;

    let client = Client::new();

    for league in leagues {
        let (name, slug) = (league.name, league.slug);

        let url = format!("https://site.api.espn.com/apis/site/v2/sports/{slug}/scoreboard");
        info!("Fetching data for {name} ({slug})");

        let request_result = client.get(url).build();

        if let Ok(request) = request_result {
            let response = client.execute(request).await;
            if let Ok(res) = response {
                let games = res.json::<ScoreboardResponse>().await.unwrap().events;
                info!("Fetched {} games for {name}", games.len());

                let cleaned_data = games.iter().map(|game| {
                    let competition = &game.competitions[0];
                    let team_one = &competition.competitors[0];
                    let team_two = &competition.competitors[1];
                    let format = "%Y-%m-%dT%H:%M%Z";
                    let datetime_utc = NaiveDateTime::parse_from_str(&game.date, format).unwrap().and_utc();

                    CleanedData {
                        league: name.clone(),
                        external_game_id: game.id.clone(),
                        link: game.links[0].href.clone(),
                        home_team: Team {
                            name: team_one.team.short_display_name.clone(),
                            logo: team_one.team.logo.clone(),
                            score: team_one.score.parse().unwrap()
                        },
                        away_team: Team { 
                            name: team_two.team.short_display_name.clone(),
                            logo: team_two.team.logo.clone(), 
                            score: team_two.score.parse().unwrap(), 
                        },
                        start_time: datetime_utc,
                        short_detail: game.status.status_type.short_detail.clone(),
                        state: game.status.status_type.state.clone(),
                    }
                }).collect::<Vec<CleanedData>>();

                let data_len = cleaned_data.len();
                for game in cleaned_data {
                    upsert_game(pool.clone(), game).await;
                }

                info!("Upserted {} games for league {name}.", data_len);
            }
        }
    }
}
