# Scrollr

## API

### Yahoo Fantasy Sports
##### Base Endpoint: /yahoo
##### User Leagues: /leagues

Authentication

 * Headers:
 ```
 Authorization: bearer <Access Token>
 Content-Type: application/json
 ```
 * Request Body: ` { "refresh_token": "<Refresh Token>" } `



Json Response :
```
{
	nba: [
		0: {
			league_key: 	"League Key",
			league_id:		0000000,
			name:			"League Name",
			url:			"https://league_url.com",
			logo_url:		"https://league_logo.com",
			draft_status: 	"Draft Status",
			num_teams: 		0,
			scoring_type: 	"Scoring Type"
			league_type: 	"public",
			current_week: 	0,
			start_week: 	0,
			end_week: 		0,
			season: 		2025,
			game_code: 		"nba"
		}
	],
	nfl: [
    	0: {
			league_key: 	"League Key",
			league_id:		0000000,
			name:			"League Name",
			url:			"https://league_url.com",
			logo_url:		"https://league_logo.com",
			draft_status: 	"Draft Status",
			num_teams: 		0,
			scoring_type: 	"Scoring Type"
			league_type: 	"public",
			current_week: 	0,
			start_week: 	0,
			end_week: 		0,
			season: 		2025,
			game_code: 		"nfl"
		}
    ]
}
```

##### League Standings: /league/{leagueKey}/standings

Authentication

 * Headers:
 ```
 Authorization: bearer <Access Token>
 Content-Type: application/json
 ```
 * Request Body: ` { "refresh_token": "<Refresh Token>" } `

Json Response :
```
{
	standings: [
    	0: {
        	team_key: "Team Key",
            team_id: 0,
            name: "Team Name",
            url: "https://team_url.com",
            team_logo: "https://team_logo.com",
            wins: 0,
            losses: 0,
            ties: 0,
            percentage: "percentage",	// Could be an empty string: ""
            games_back: 0.0,
            points_for: 0.0,
            points_against: 0.0
        }
    ]
}
```

##### Team Roster: /team/{teamKey}/roster

Query Parameters
```
sport=<sport>			// This is required
						// Currently supports: 
                        // nfl or football,
                        // nba or basketball
                        
date=<year-month-day>	// Optional
```

Authentication

* Headers:
```
Authorization: bearer <Access Token>
Content-Type: application/json
```
* Request Body: ` { "refresh_token": "<Refresh Token>" } `

Json Response :
```
{
	roster: [
    	id: 00000,
        key: "Player Key",
        name: "Player Name",
        firstName: "First Name",
        lastName: "Last Name",
        teamAbbr: "Team Abbreviation",
        teamFullName: "Full Team Name",
        uniformNumber: "00",
        position: "Position",
        selectedPosition: "Selected Position",
        eligiblePositions: [
        	0: "Eligible Position"
        ],
        imageUrl: "https://player_image.com",
        headshot: "https://player_image.com",	// Believed to always be the same Url as imageUrl
        isUndroppable: true,
        positionType: O,
        stats: [
        	0: {
            	name: "stat name",
                value: 0
            }
        ],
        playerPoints: {
        	coverage_type: "week",
            week: 0 || null,
            date: "year-month-day" || null,
            total: 00.00
        }
    ]
}
```