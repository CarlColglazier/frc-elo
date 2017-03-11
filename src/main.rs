extern crate curl;
extern crate rustc_serialize;
extern crate dotenv;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate csv;
extern crate pbr;
#[macro_use] extern crate clap;

mod tba;
mod schema;
mod models;
mod elo;
mod glicko;

use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use diesel::prelude::*;
use models::*;
use elo::Teams;
use glicko::GlickoTeams;
use std::{thread, str, env};
use std::fs::OpenOptions;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::clone::Clone;
use pbr::ProgressBar;
use schema::matches::dsl::*;
use schema::events::dsl::*;
use std::cmp::Ordering;
use clap::App;

const FIRST_YEAR: i32 = 2002;
const NEXT_YEAR: i32 = 2018;
const CURRENT_YEAR: i32 = NEXT_YEAR - 1;

#[derive(Clone)]
struct RequestData {
    events: Vec<models::EventJSON>,
    matches: Vec<models::GameMatch>,
}

impl RequestData {
    pub fn new() -> RequestData {
        RequestData {
            events: Vec::new(),
            matches: Vec::new(),
        }
    }
}

#[derive(RustcDecodable)]
struct HistoryRecord {
    url: String,
    time: String,
}

fn open_history() -> HashMap<String, String> {
    let f = OpenOptions::new().create(true).write(true).open("tba_history.csv").unwrap();
    drop(f);
    let mut map: HashMap<String, String> = HashMap::new();
    let mut rdr = csv::Reader::from_file("tba_history.csv").unwrap();
    for record in rdr.decode() {
        let record: HistoryRecord = match record {
            Ok(r) => r,
            Err(e) => {
                panic!("ERROR: {}", e.description());
                //continue;
            },
        };
        map.insert(record.url, record.time);
    }
    return map;
}

fn write_history(map: &HashMap<String, String>) {
    let mut wtr = csv::Writer::from_file("tba_history.csv").unwrap();
    for record in map.iter() {
        let _ = wtr.encode(record);
    }
}

fn db_connect() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").
        expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error ocnnecting to {}", database_url))
}

fn setup() {
    let mut threads = Vec::new();
    let history = Arc::new(Mutex::new(open_history()));
    let request_data: Arc<Mutex<RequestData>> = Arc::new(Mutex::new(RequestData::new()));
    //println!("Syncing data");
    
    for i in 2002..NEXT_YEAR {
        let request_data = request_data.clone();
        let history = history.clone();
        threads.push(thread::spawn(move || {
            let mut info = RequestData::new();
            let url = format!("events/{}", i);
            let mut last_time = String::new();
            
            {
                let history = history.lock()
                    .expect("Could not lock history for getting event time");
                match history.get(&url) {
                    Some(date) => last_time.push_str(&date),
                    None => {},
                };
            }
            let response = tba::request(&url, &last_time);
            if response.code != 200 && i < NEXT_YEAR - 1 {
                return;
            }
            {
                let mut history = history.lock()
                    .expect("Could not lock history for setting event time");
                history.insert(url, response.last_modified.trim().to_string());
            }
            if response.data.len() > 0 {
                let data_str = str::from_utf8(&response.data)
                    .expect("Could not load data string");
                let mut event_list: Vec<models::EventJSON> = serde_json::from_str(&data_str)
                    .expect("Could not parse events JSON");
                info.events.append(&mut event_list);
                let mut bar = ProgressBar::new(info.events.len() as u64);
                for event in &info.events {
                    bar.inc();
                    let url = format!("event/{}/matches", event.key);
                    let mut last_time = String::new();
                    {
                        let history = history.lock()
                            .expect("Could not get history for match reading");
                        match history.get(&url) {
                            Some(date) => last_time.push_str(&date),
                            None => {},
                        };
                    }
                    let response = tba::request(&url, &last_time);
                    if response.code != 200 {
                        continue;
                    }
                    println!("Updating {}", url);
                    {
                        let mut history = history.lock()
                            .expect("Could not get history for match writing");
                        history.insert(url, response.last_modified.trim().to_string());
                    }
                    let data_str = str::from_utf8(&response.data)
                        .expect("Could not load match data string");
                    let mut game_matches: Vec<models::GameMatch> =
                        match serde_json::from_str(&data_str) {
                            Ok(m) => m,
                            Err(e) => {
                                println!("Error: {}", e.description());
                                continue;
                            },
                        };
                    info.matches.append(&mut game_matches);
                }
                let mut request_data = request_data.lock()
                    .expect("Could not lock request data");
                request_data.events.append(&mut info.events);
                request_data.matches.append(&mut info.matches);
            }
        }));
    }
    for child in threads {
        let _ = child.join();
    }
    let result = request_data.lock().unwrap();
    //println!("Found {} new events and {} new matches", result.events.len(), result.matches.len());
    //println!("Updating database");
    let conn = db_connect();
    let new_events: Vec<NewEvent> = result.events.iter().map(|x| prepare_event(x)).collect();
    diesel::insert_or_replace(&new_events).into(events).execute(&conn)
        .expect("Could not insert events");
    let new_matches: Vec<NewMatch> = result.matches.iter().filter_map(|x| prepare_match(x)).collect();
    diesel::insert_or_replace(&new_matches).into(matches).execute(&conn)
        .expect("Could not insert mathes");
    let history = history.lock().unwrap();
    write_history(&history);
}

fn get_matches() -> (Vec<Event>, Vec<Vec<Matche>>) {
    //println!("Calculating Elo rankings");
    let conn = db_connect();
    let event_list = events
        .filter(official.eq(1))
        .order(start_date)
    //.order(schema::events::dsl::id)
        .load::<Event>(&conn).expect("Could not query events");
    let event_match_list = Matche::belonging_to(&event_list)
        .filter(red_score.gt(-1))
        .filter(blue_score.gt(-1))
        .order(match_number)
        .load::<Matche>(&conn)
        .expect("Could not query matches")
        .grouped_by(&event_list);
    let mut final_list: Vec<Vec<Matche>> = Vec::new();
    for mut event in event_match_list {
        event.sort_by(|a, b| {
            let a_level = match a.comp_level.as_ref() {
                "qm" => 0,
                "qf" => 1,
                "sf" => 2,
                "f" => 3,
                _ => 100,
            };
            let b_level = match b.comp_level.as_ref() {
                "qm" => 0,
                "qf" => 1,
                "sf" => 2,
                "f" => 3,
                _ => 100,
            };
            if a_level > b_level {
                return Ordering::Greater
            } else if a_level < b_level {
                return Ordering::Less;
            }
            if a.match_number > b.match_number {
                return Ordering::Greater;
            } else if a.match_number < b.match_number {
                return Ordering::Less;
            }
            return Ordering::Equal;
        });
        final_list.push(event);
    }
    return (event_list, final_list);
}
//println!("Actual,Predicted");

fn elo (k: f64, carry_over: f64) {
    let mut team_list = Teams::new(k, carry_over);
    let mut current_year = FIRST_YEAR;
    let (_, event_match_list) = get_matches();
    for event in event_match_list {
        if event.len() < 1 {
            continue;
        }
        if !event.first().unwrap().id.contains(&format!("{}", current_year)) {
            team_list.new_year();
            current_year += 1;
        }
        for m in event {
            team_list.process_match(&m);
        }
    }
    let brier = team_list.brier / team_list.total as f64;
    println!("Brier: {}", brier);
    let mut teams = Vec::new();
    for (key, val) in team_list.table {
        teams.push((key, val));
    }
    teams.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    let mut i = 1;
    for (key, val) in teams {
        println!("{}. {}    {}", i, key, val);
        i += 1;
    }
}

fn glicko(year: i32) -> GlickoTeams {
    let mut team_list = GlickoTeams::new();
    let mut current_year = FIRST_YEAR;
    let (event_list, match_list) = get_matches();
    for event in &match_list {
        if event.len() < 1 {
            continue;
        }
        let ref current_event_id = event.first().unwrap().event_id;
        //println!("{}", current_event_id);
        if !current_event_id.contains(&format!("{}", current_year)) {
            if current_year == year {
                break;
            }
            team_list.new_year();
            current_year += 1;
        }
        let mut week_iter = event_list.iter().filter(|x| x.id.contains(current_event_id));
        let current_week = week_iter.next().expect("Week event")
            .week;
        team_list.start_event(current_week);
        for m in event {
            team_list.process_match(&m);
            if m.id.ends_with("qf1m1") || m.match_number % 20 == 0 {
                team_list.finish_event();
            }
        }
        team_list.finish_event();
        
    }
    return team_list;
}

fn main() {
    dotenv().ok();
    let yaml = load_yaml!("cli.yaml");
    let cli_matches = App::from_yaml(yaml).get_matches();
    if let Some(_) = cli_matches.subcommand_matches("sync") {
        setup();
    }
    if let Some(_) = cli_matches.subcommand_matches("elo") {
        elo(16f64, 0.8f64);
    }
    if let Some(m) = cli_matches.subcommand_matches("glicko") {
        let year: i32 = match m.value_of("year") {
            Some(y) => y.parse().unwrap_or(NEXT_YEAR - 1),
            None => NEXT_YEAR - 1,
        };
        let team_list = glicko(year);
        let mut teams = Vec::new();
        for (key, val) in &team_list.table {
            if val.glicko.deviation < 140f64 || m.is_present("all") {
                teams.push((key.clone(), val.glicko.clone()));
            }
        }
        let brier = team_list.brier / team_list.total as f64;
        println!("Brier: {}", brier);
        teams.sort_by(|x, y| y.1.rating.partial_cmp(&x.1.rating).unwrap());
        let mut i = 1;
        for (key, val) in teams {
            println!("{:>4}. {:<7}  {:^4}  ({:>4},{:>4}) [{}]", i, key,
                     val.rating as i32,
                     (val.rating - 1.96f64 * val.deviation) as i32,
                     (val.rating + 1.96f64 * val.deviation) as i32,
                     val.deviation as i32);
            i += 1;
        }

    }
    if let Some(m) = cli_matches.subcommand_matches("predict") {
        let red = m.value_of("red").unwrap().split(" ");
        let red_list: Vec<String> = red.map(|x| x.to_owned()).collect();
        let blue = m.value_of("blue").unwrap().split(" ");
        let blue_list: Vec<String> = blue.map(|x| x.to_owned()).collect();
        let mut team_list = glicko(CURRENT_YEAR);
        println!("{:?} {:?}", red_list, blue_list);
        let red_glicko = team_list.average(&red_list);
        let blue_glicko = team_list.average(&blue_list);
        println!("{:?}", red_glicko.predict(&blue_glicko));
    }
    if let Some(m) = cli_matches.subcommand_matches("prob") {
        let event_key = m.value_of("event").unwrap();
        let mut team_list = glicko(CURRENT_YEAR);
        let conn = db_connect();
        let match_list = matches
            .filter(event_id.eq(event_key))
            .filter(red_score.eq(-1))
            .filter(blue_score.eq(-1))
            .order(match_number)
            .load::<Matche>(&conn)
            .expect("matches");
        for m in &match_list {
            let red_glicko = team_list.average(&m.get_red());
            let blue_glicko = team_list.average(&m.get_blue());
            let prediction = red_glicko.predict(&blue_glicko);
            let red_teams = m.get_red().join(" ");
            let blue_teams = m.get_blue().join(" ");
            println!("{}{:<2} ({:.5}) {:<24} vs. {:<24} ({:.5})",
                     m.comp_level, m.match_number, prediction, red_teams,
                     blue_teams, 1f64 - prediction);
        }
    }
    if let Some(m) = cli_matches.subcommand_matches("estimate") {
        let event_key = m.value_of("event").unwrap();
        let mut team_list = glicko(CURRENT_YEAR);
        println!("{:?}", event_key);
        let conn = db_connect();
        let match_list = matches
            .filter(event_id.eq(event_key))
            .filter(comp_level.eq("qm"))
            .load::<Matche>(&conn)
            .expect("matches");
        let mut rankings: HashMap<String, f64> = HashMap::new();
        for m in &match_list {
            let completed = m.blue_score != -1 && m.red_score != -1;
            let red_glicko = team_list.average(&m.get_red());
            let blue_glicko = team_list.average(&m.get_blue());
            let prediction = red_glicko.predict(&blue_glicko);
            for team in &m.get_red() {
                let mut ranking = rankings.entry(team.to_owned()).or_insert(0f64);
                if completed {
                    *ranking += 2f64 * m.actual_r();
                } else {
                    *ranking += 2f64 * prediction;
                }
            }
            for team in &m.get_blue() {
                let mut ranking = rankings.entry(team.to_owned()).or_insert(0f64);
                if completed {
                    *ranking += 2f64 * m.actual_b();
                } else {
                    *ranking += 2f64 * (1f64 - prediction);
                }
            }
        }
        let mut teams = Vec::new();
        for (team, val) in rankings {
            teams.push((team, val));
        }
        teams.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
        for (key, val) in teams {
            println!("{} {}", key, val);
        }
    }
}
