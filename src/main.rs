extern crate curl;
extern crate rustc_serialize;
extern crate dotenv;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;
extern crate csv;
//extern crate pbr;

mod tba;
mod schema;
mod models;

use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use diesel::prelude::*;
use models::*;
use schema::*;
use std::{thread, str, env};
use std::fs::OpenOptions;
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::clone::Clone;
//use pbr::ProgressBar;

const CURRENT_YEAR: i32 = 2017;

#[derive(Clone)]
struct RequestData {
    events: Vec<models::EventJSON>,
    matches: Vec<models::GameMatch>,
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
    let request_data: Arc<Mutex<RequestData>> =
        Arc::new(Mutex::new(RequestData {
            events: Vec::new(),
            matches: Vec::new(),
        }));
    println!("Syncing data");
    for i in 2002..CURRENT_YEAR {
        let request_data = request_data.clone();
        let history = history.clone();
        threads.push(thread::spawn(move || {
            let mut info = RequestData {
                events: Vec::new(),
                matches: Vec::new(),
            };
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
            if response.code != 200 {
                return;
            }
            let data = response.data;
            {
                let mut history = history.lock()
                    .expect("Could not lock history for setting event time");
                history.insert(url, response.last_modified.trim().to_string());
            }
            if data.len() > 0 {
                let data_str = str::from_utf8(&data)
                    .expect("Could not load data string");
                let mut events: Vec<models::EventJSON> = serde_json::from_str(&data_str)
                    .expect("Could not parse events JSON");
                info.events.append(&mut events);
                for event in &info.events {
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
                    let data = response.data;
                    {
                        let mut history = history.lock()
                            .expect("Could not get history for match writing");
                        history.insert(url, response.last_modified.trim().to_string());
                    }
                    let data_str = str::from_utf8(&data)
                        .expect("Could not load match data string");
                    let mut game_matches: Vec<models::GameMatch> = match serde_json::from_str(&data_str) {
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
    println!("Found {} new events and {} new matches", result.events.len(), result.matches.len());
    println!("Updating database");
    let conn = db_connect();
    let new_events: Vec<NewEvent> = result.events.iter().map(|x| prepare_event(x)).collect();
    diesel::insert_or_replace(&new_events).into(events::table).execute(&conn)
        .expect("Could not insert events");
    let new_matches: Vec<NewMatch> = result.matches.iter().filter_map(|x| prepare_match(x)).collect();
    diesel::insert_or_replace(&new_matches).into(matches::table).execute(&conn)
        .expect("Could not insert mathes");
    let history = history.lock().unwrap();
    write_history(&history);
    println!("Calculating Elo rankings");

    let events = events::table.load::<Event>(&conn).expect("Could not query events");
    let matches = Matche::belonging_to(&events).load::<Matche>(&conn)
        .expect("Could not query matches");
    println!("{} matches found.", matches.len());
}


fn main() {
    //fs::create_dir_all(tba::TBA_DATA_DIR).unwrap();
    setup();
}
