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

use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use diesel::prelude::*;
use models::*;
use elo::Teams;
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
const CURRENT_YEAR: i32 = 2018;

const VERSION: &'static str = "0.0.0";

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
    
    for i in 2002..CURRENT_YEAR {
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
            if response.code != 200 && i < CURRENT_YEAR - 1 {
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

fn calculate (k: f64, carry_over: f64) {
    //println!("Calculating Elo rankings");
    let mut team_list = Teams::new(k, carry_over);
    let conn = db_connect();
    let event_list = events
        .filter(official.eq(1))
        .order(start_date)
        .order(event_type)
        .order(schema::events::dsl::id)
        .load::<Event>(&conn).expect("Could not query events");
    let event_match_list = Matche::belonging_to(&event_list)
        .filter(red_score.gt(-1))
        .filter(blue_score.gt(-1))
        .order(match_number)
        .load::<Matche>(&conn)
        .expect("Could not query matches")
        .grouped_by(&event_list);
    //println!("Actual,Predicted");
    let mut current_year = FIRST_YEAR;
    for mut event in event_match_list {
        if event.len() < 1 {
            continue;
        }
        if !event.first().unwrap().id.contains(&format!("{}", current_year)) {
            team_list.new_year();
            current_year += 1;
        }
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


fn main() {
    let yaml = load_yaml!("cli.yaml");
    let cli_matches = App::from_yaml(yaml).get_matches();
    if let Some(m) = cli_matches.subcommand_matches("sync") {
        setup();
    }
    if let Some(m) = cli_matches.subcommand_matches("rank") {
        calculate(12f64, 0.9f64);
    }
}
