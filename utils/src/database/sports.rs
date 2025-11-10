use std::sync::Arc;

use chrono::Utc;
use log::{error, info};
use serde::Deserialize;
use sqlx::{PgPool, query};

#[derive(Deserialize, Clone)]
pub struct LeagueConfigs {
    pub name: String,
    pub slug: String,
}

#[derive(Debug)]
pub struct CleanedData {
    pub league: String,
    pub external_game_id: String,
    pub link: String,
    pub home_team: Team,
    pub away_team: Team,
    pub start_time: chrono::DateTime<Utc>,
    pub short_detail: String,
    pub state: String,
}

#[derive(Debug)]
pub struct Team {
    pub name: String,
    pub logo: String,
    pub score: i32
}

pub async fn create_tables(pool: Arc<PgPool>) {
    let statement = "
        CREATE TABLE IF NOT EXISTS games (
            id SERIAL PRIMARY KEY,
            league VARCHAR(50) NOT NULL,
            external_game_id VARCHAR(100) NOT NULL,
            link VARCHAR(500),
            home_team_name VARCHAR(100) NOT NULL,
            home_team_logo VARCHAR(500),
            home_team_score INTEGER,
            away_team_name VARCHAR(100) NOT NULL,
            away_team_logo VARCHAR(500),
            away_team_score INTEGER,
            start_time TIMESTAMP WITH TIME ZONE NOT NULL,
            short_detail VARCHAR(200),
            state VARCHAR(50) NOT NULL,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(league, external_game_id)
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

pub async fn clear_tables(pool: Arc<PgPool>, leagues: Vec<LeagueConfigs>) {
    let league_names: Vec<String> = leagues.iter().map(|league| league.name.clone()).collect();

    if league_names.is_empty() {
        return;
    }

    let placeholders = (1..=league_names.len())
        .map(|i| format!("${}", i))
        .collect::<Vec<String>>()
        .join(", ");

    let conn = pool.acquire().await;

    let statement = format!("DELETE FROM games WHERE league IN ({});", placeholders);

    if let Ok(mut connection) = conn {
        let mut db_query = query(&statement);

        for name in &league_names {
            db_query = db_query.bind(name);
        }

        let _ = db_query
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));

        info!("All rows with league_type {:?} have been deleted", league_names);
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}

pub async fn upsert_game(pool: Arc<PgPool>, game: CleanedData) {
    let statement = "
        INSERT INTO games (
            league,
            external_game_id,
            link,
            home_team_name,
            home_team_logo,
            home_team_score,
            away_team_name,
            away_team_logo,
            away_team_score,
            start_time,
            short_detail,
            state
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        ON CONFLICT (league, external_game_id)
        DO UPDATE
            SET link             = EXCLUDED.link,
                home_team_name   = EXCLUDED.home_team_name,
                home_team_logo   = EXCLUDED.home_team_logo,
                home_team_score  = EXCLUDED.home_team_score,
                away_team_name   = EXCLUDED.away_team_name,
                away_team_logo   = EXCLUDED.away_team_logo,
                away_team_score  = EXCLUDED.away_team_score,
                start_time       = EXCLUDED.start_time,
                short_detail     = EXCLUDED.short_detail,
                state            = EXCLUDED.state,
                updated_at       = CURRENT_TIMESTAMP;
    ";

    let conn = pool.acquire().await;

    if let Ok(mut connection) = conn {
        let _ = query(statement)
            .bind(&game.league)
            .bind(game.external_game_id)
            .bind(game.link)
            .bind(game.home_team.name)
            .bind(game.home_team.logo)
            .bind(game.home_team.score)
            .bind(game.away_team.name)
            .bind(game.away_team.logo)
            .bind(game.away_team.score)
            .bind(game.start_time)
            .bind(game.short_detail)
            .bind(game.state)
            .execute(&mut *connection)
            .await
            .inspect_err(|e| error!("Execution Error: {}", e));
    } else {
        error!("Connection Error: Failed to acquire a connection from the pool");
    }
}