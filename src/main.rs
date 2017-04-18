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
extern crate rand;
#[macro_use] extern crate tera;
extern crate chrono;
extern crate probability;

mod tba;
mod schema;
mod models;
mod elo;

use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use diesel::prelude::*;
use models::*;
use elo::Teams;
use tba::TeamEventRanking;
use std::{thread, str, env};
use std::fs::OpenOptions;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::clone::Clone;
use schema::matches::dsl::*;
use schema::events::dsl::*;
use std::cmp::Ordering;
use clap::App;
use rand::Rng;
use tera::Context;
use chrono::offset::utc::UTC;
use chrono::datetime::DateTime;

/// The first year for which data exists.
/// This is used by the `sync` command as the first
/// year from which events are fetched.
const FIRST_YEAR: i32 = 2002;
/// The current year or the last year from which
/// events are fetched.
const CURRENT_YEAR: i32 = 2017;
/// Based off of the `CURRENT_YEAR`. Used in several
/// loops.
const NEXT_YEAR: i32 = CURRENT_YEAR + 1;
/// The number of simulations to run when modeling. 
const EST_RUNS: usize = 10000;

/// Holds events and matches which will eventually
/// need to be added to the database.
#[derive(Clone)]
struct RequestData {
    /// A list of event responses from parsed JSON.
    events: Vec<models::EventJSON>,
    /// A list of match responses from parsed JSON.
    matches: Vec<models::GameMatch>,
}

impl RequestData {
    /// Create an empty instance of `RequestData`.
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

#[derive(Serialize, Clone)]
struct TableEntry {
    team: String,
    rating: f64,
}

/// Get the hash map containing the URLs and time strings.
/// Values are read form a CSV file named `tba_history.csv`.
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

/// Given a hash map, record to a CSV file named `tba_history.csv`
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
    //let conn = db_connect();
    let conn = Arc::new(Mutex::new(db_connect()));
    //let request_data: RequestData = RequestData::new();
    
    for i in 2002..NEXT_YEAR {
        let history = history.clone();
        let conn = conn.clone();
        threads.push(thread::spawn(move || {
            let mut info = RequestData::new();
            if let Some(mut event_list) = tba::get_events(history.clone(), i) {
                info.events.append(&mut event_list);
                let mut event_threads = Vec::new();
                for i in 0..5 {
                    let event_list = info.events.clone();
                    let history = history.clone();
                    let conn = conn.clone();
                    event_threads.push(thread::spawn(move || {
                        let mut result = RequestData::new();
                        for j in 0..event_list.len() / 5 + 1 {
                            let index = i + 5 * j;
                            if index >= event_list.len() {
                                break;
                            }
                            if let Some(mut em) = tba::get_event_matches(history.clone(),
                                                                         &event_list[index].key) {
                                result.events.push(event_list[index].clone());
                                result.matches.append(&mut em);
                            }
                        }
                        let conn = conn.lock().expect("Database connection");
                        let new_events: Vec<NewEvent> = result.events.iter()
                            .map(|x| prepare_event(x)).collect();
                        diesel::insert_or_replace(&new_events)
                            .into(events).execute(&*conn)
                            .expect("Could not insert events");
                        if result.matches.len() > 0 {
                            let new_matches: Vec<NewMatch> = result.matches.iter()
                                .filter_map(|x| prepare_match(x)).collect();
                            diesel::insert_or_replace(&new_matches).into(matches).execute(&*conn)
                                .expect("Could not insert mathes");
                        }
                    }));
                }
                for child in event_threads {
                    let _ = child.join();
                }
            }
        }));
    }
    for child in threads {
        let _ = child.join();
    }
    let history = history.lock().unwrap();
    write_history(&history);
}

fn get_matches() -> (Vec<Event>, Vec<Vec<Matche>>) {
    let conn = db_connect();
    let event_list = events
        .filter(official.eq(1))
        .filter(event_type.lt(99))
        .filter(start_date.gt("2008"))
        .order(start_date)
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

fn get_week_events(week_num: i32) -> Vec<Event> {
    let conn = db_connect();
    return events
        .filter(official.eq(1))
        .filter(event_type.lt(99))
        .filter(start_date.gt(&format!("{}", CURRENT_YEAR)))
        .filter(week.eq(week_num))
        .load::<Event>(&conn).expect("Events");
}

fn elo (k: f64, carry_over: f64, brier_ret: &mut f64) -> Teams {
    let mut team_list = Teams::new(k, carry_over,FIRST_YEAR as usize);
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
    //println!("Brier: {}", brier);
    //println!("BSS: {}", 1f64 - brier / 0.25f64);
    //println!("Predicted {} of {}, {}", team_list.wins_correct, team_list.total,
    //team_list.wins_correct as f64 / team_list.total as f64);
    *brier_ret = brier;
    return team_list;
}

#[derive(Serialize, Clone)]
struct EventTable {
    key: String,
    name: String,
    entries: Vec<TableEntry>,
}

impl EventTable {
    fn new() -> EventTable {
        EventTable {
            key: String::new(),
            name: String::new(),
            entries: Vec::new(),
        }
    }
}

fn main() {
    dotenv().ok();
    let yaml = load_yaml!("cli.yaml");
    let cli_matches = App::from_yaml(yaml).get_matches();
    if let Some(_) = cli_matches.subcommand_matches("sync") {
        setup();
    }
    if let Some(m) = cli_matches.subcommand_matches("elo") {
        let mut brier = 0.0f64;
        let mut team_list = elo(15f64, 0.8f64, &mut brier);
        let mut teams = Vec::new();
        for (key, val) in &team_list.table {
            teams.push(TableEntry {
                team: key.to_owned(),
                rating: val.to_owned(),
            });
        }
        teams.sort_by(|x, y| y.rating.partial_cmp(&x.rating).unwrap());
        if m.is_present("html") {
            let tera = compile_templates!("templates/**/*");
            let mut context = Context::new();
            context.add("ratings", &teams);
            let mut event_contexts = Vec::new();
            for e in get_week_events(7) {
                let mut event_entry = EventTable::new();
                event_entry.key.push_str(&e.id);
                event_entry.name.push_str(&e.name);
                for team in tba::get_event_teams(&e.id).unwrap() {
                    event_entry.entries.push(TableEntry {
                        team: team.clone(),
                        rating: team_list.get(&team),
                    });
                    //let rating = teams.
                    //println!("{} {}", &team, team_list.get(&team));
                }
                event_entry.entries.sort_by(|x, y| y.rating.partial_cmp(&x.rating).unwrap());
                if event_entry.entries.len() > 0 {
                    event_contexts.push(event_entry);
                }
            }
            context.add("events", &event_contexts);
            context.add("timestamp", &UTC::now().to_rfc2822());
            context.add("brier", &brier);
            let rendered = tera.render("index.html", &context).unwrap();
            
            println!("{}", rendered);
            return;
        } else {
            let mut i = 1;
            for t in teams {
                println!("{:-4}. {:<8} {:<.3}", i, t.team, t.rating);
                i += 1;
            }
        }
    }
}
