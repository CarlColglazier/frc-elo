use super::schema::*;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::insert;

#[derive(Deserialize, Queryable, Debug)]
pub struct Event {
    pub key: String,
    pub name: String,
    pub event_type: usize,
    pub official: Option<bool>,
    pub start_date: String,
}

#[derive(Insertable)]
#[table_name="events"]
pub struct NewEvent<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub event_type: i32,
    pub official: i32,
    pub start_date: &'a str,
}

pub fn create_event(conn: &SqliteConnection, event: &Event)  -> QueryResult<usize> {
    construct_event(conn, &event.key, &event.name, event.event_type as i32,
                    event.official.map(|s| match s {
                        true => 1,
                        false => 0,
                    }).unwrap_or(0), &event.start_date)
}

fn construct_event<'a>(conn: &SqliteConnection, key: &'a str, name: &'a str,
                       event_type: i32, official: i32, start_date: &'a str) -> QueryResult<usize> {
    let new_event = NewEvent {
        key: key,
        name: name,
        event_type: event_type,
        official: official,
        start_date: start_date,
    };
    insert(&new_event).into(events::table).execute(conn)
}

#[derive(Deserialize, Queryable, Debug)]
pub struct Alliances {
    pub red: Alliance,
    pub blue: Alliance,
}

#[derive(Deserialize, Queryable, Debug)]
pub struct Alliance {
    pub score: i32,
    pub teams: Vec<String>
}

#[derive(Deserialize, Queryable, Debug)]
pub struct GameMatch {
    pub key: String,
    pub comp_level: String,
    pub match_number: i32,
    pub set_number: i32,
    pub event_key: String,
    pub alliances: Alliances,
}

#[derive(Insertable)]
#[table_name="matches"]
pub struct NewMatch<'a> {
    pub key: &'a str,
    pub comp_level: &'a str,
    pub match_number: i32,
    pub set_number: i32,
    pub event_key: &'a str,
    pub red_score: i32,
    pub blue_score: i32,
    pub red1: &'a str,
    pub red2: &'a str,
    pub red3: &'a str,
    pub blue1: &'a str,
    pub blue2: &'a str,
    pub blue3: &'a str,
}

pub fn create_match(conn: &SqliteConnection, game_match: &GameMatch) -> QueryResult<usize> {
    if game_match.alliances.red.teams.len() < 2 || game_match.alliances.blue.teams.len() < 2 {
        return Ok(0);
    }
    let empty = String::new();
    construct_match(conn, &game_match.key, &game_match.comp_level,
                    game_match.match_number, game_match.set_number,
                    &game_match.key, game_match.alliances.red.score,
                    game_match.alliances.blue.score,
                    &game_match.alliances.red.teams[0],
                    &game_match.alliances.red.teams[1],
                    &game_match.alliances.red.teams.get(2).unwrap_or(&empty),
                    &game_match.alliances.blue.teams[0],
                    &game_match.alliances.blue.teams[1],
                    &game_match.alliances.blue.teams.get(2).unwrap_or(&empty))
}

fn construct_match<'a>(conn: &SqliteConnection, key: &'a str,
                       comp_level: &'a str, match_number: i32, set_number: i32,
                       event_key: &'a str,
                       red_score: i32, blue_score: i32, red1: &'a str,
                       red2: &'a str, red3: &'a str, blue1: &'a str, blue2: &'a str,
                       blue3: &'a str) -> QueryResult<usize> {
    let new_match = NewMatch {
        key: key,
        comp_level: comp_level,
        match_number: match_number,
        set_number: set_number,
        event_key: event_key,
        red_score: red_score,
        blue_score: blue_score,
        red1: red1,
        red2: red2,
        red3: red3,
        blue1: blue1,
        blue2: blue2,
        blue3: blue3,
    };
    insert(&new_match).into(matches::table).execute(conn)
}


/*
#[derive(Deserialize, Queryable)]
pub struct Team {
team_number: u16,
nickname: String,
key: String,
}
*/
