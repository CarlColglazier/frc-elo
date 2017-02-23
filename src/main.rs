extern crate curl;
extern crate rustc_serialize;
extern crate filetime;
extern crate chrono;
extern crate dotenv;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
#[macro_use] extern crate diesel;
#[macro_use] extern crate diesel_codegen;

mod tba;
mod schema;
mod models;

use std::fs;
use diesel::sqlite::SqliteConnection;
use dotenv::dotenv;
use diesel::prelude::*;
use std::env;
use std::str;
use models::*;
use std::thread;
use std::time;
use std::io::BufReader;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::error::Error;

const CURRENT_YEAR: i32 = 2017;

pub fn db_connect() -> SqliteConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").
        expect("DATABASE_URL must be set");
    SqliteConnection::establish(&database_url)
        .expect(&format!("Error ocnnecting to {}", database_url))
}

fn setup() {
    let mut threads = Vec::new();
    for i in 2002..CURRENT_YEAR {
        threads.push(thread::spawn(move || {
            let url = format!("events/{}", i);
            let data = tba::request(&url);
            if data.len() > 0 {
                let data_str = str::from_utf8(&data).unwrap();
                let events: Vec<models::Event> = serde_json::from_str(&data_str).unwrap();
                for event in events {
                    let url = format!("event/{}/matches", event.key);
                    let data = tba::request(&url);
                }
            }
        }));
    }
    for child in threads {
        let _ = child.join();
    }
    let conn = db_connect();
    let temp = File::open("temp.txt").expect("Could not open temp.txt");
    let mut reader = BufReader::new(temp);
    let mut line = String::new();
    for l in reader.lines() {
        let line = l.unwrap();
        if line.contains("events") {
            let mut event_file = match File::open(&line) {
                Ok(file) => file,
                Err(e) => panic!("Could not find \"{}\": {}", line, e.description()),
            };
            let mut file_buffer = String::new();
            event_file.read_to_string(&mut file_buffer);
            let events: Vec<models::Event> = serde_json::from_str(&file_buffer).unwrap();
            for event in events {
                match create_event(&conn, &event) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error: {}", e.description());
                        break;
                    },
                };
            }
        } else if line.contains("event") {
            let mut match_file = File::open(&line).expect(&format!("Could not find {}", line));
            let mut file_buffer = String::new();
            match_file.read_to_string(&mut file_buffer);
            let matches: Vec<models::GameMatch> = serde_json::from_str(&file_buffer).unwrap();
            for m in matches {
                match create_match(&conn, &m) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Error: {}", e.description());
                        thread::sleep(time::Duration::from_millis(50));
                    },
                };
            }
        }
    }
    fs::remove_file("temp.txt");
}


fn main() {
    fs::create_dir_all(tba::TBA_DATA_DIR).unwrap();
    setup();
}
