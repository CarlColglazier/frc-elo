use curl::easy::{Easy, List};
use std::str;
use std::env;
use models;
use std::sync::Arc;
use std::sync::Mutex;
use std::collections::HashMap;
use serde_json;
use std::error::Error;
use CURRENT_YEAR;

pub struct Response {
    pub code: u32,
    pub data: Vec<u8>,
    pub last_modified: String,
}

pub fn get_events(history: Arc<Mutex<HashMap<String, String>>>,
                  year: i32) -> Option<Vec<models::EventJSON>> {
    let url = format!("events/{}", year);
    let mut last_time = String::new();
    if year != CURRENT_YEAR {
        let history = history.lock()
            .expect("Could not lock history for getting event time");
        match history.get(&url) {
            Some(date) => last_time.push_str(&date),
            None => {},
        };
    }
    let response = request(&url, &last_time);
    if response.code != 200 { //&& year < CURRENT_YEAR {
        return None;
    }
    {
        let mut history = history.lock()
            .expect("Could not lock history for setting event time");
        history.insert(url, response.last_modified.trim().to_string());
    }
    if response.data.len() > 0 {
        let data_str = str::from_utf8(&response.data)
            .expect("Could not load data string");
        let event_list: Vec<models::EventJSON> = serde_json::from_str(&data_str)
            .expect("Could not parse events JSON");
        return Some(event_list);
    }
    return None;
}

pub fn get_event_matches(history: Arc<Mutex<HashMap<String, String>>>,
                         key: &str) -> Option<Vec<models::GameMatch>> {
    let url = format!("event/{}/matches/simple", key);
    let mut last_time = String::new();
    {
        let history = history.lock()
            .expect("Could not get history for match reading");
        match history.get(&url) {
            Some(date) => last_time.push_str(&date),
            None => {},
        };
    }
    let response = request(&url, &last_time);
    if response.code != 200 {
        return None;
    }
    println!("Updating {}", url);
    {
        let mut history = history.lock()
            .expect("Could not get history for match writing");
        history.insert(url, response.last_modified.trim().to_string());
    }
    let data_str = str::from_utf8(&response.data)
        .expect("Could not load match data string");
    match serde_json::from_str(&data_str) {
        Ok(m) => return Some(m),
        Err(e) => {
            println!("Error: {}", e.description());
            return None;
        },
    }
}

pub fn request(url_ext: &str, date: &str) -> Response {
    let request_url = format!("https://www.thebluealliance.com/api/v3/{}", url_ext);
    let mut easy = Easy::new();
    let mut list = List::new();
    let mut data = Vec::new();
    list.append("X-TBA-App-Id: Carl Colglazier:FRC ELO:0.0.0").unwrap();
    list.append(&format!("X-TBA-Auth-Key: {}", env::var("TBA_KEY")
                         .expect("Auth Key"))).expect("Add auth key");
    if date.len() > 0 {
        let time_header = format!("If-Modified-Since: {}", date);
        list.append(&time_header).unwrap();
    }
    easy.http_headers(list).unwrap();
    easy.url(&request_url).unwrap();
    let mut headers = String::new();
    {
        let mut transfer = easy.transfer();
        transfer.write_function(|new_data| {
            data.extend_from_slice(new_data);
            Ok(new_data.len())
        }).unwrap();
        transfer.header_function(|header| {
            let s = str::from_utf8(header).unwrap().to_string();
            if s.starts_with("Last-Modified: ") {
                headers.push_str(&s[15..]);
            }
            true
        }).unwrap();
        transfer.perform().unwrap();
    }
    let code = easy.response_code().unwrap();
    return Response {
        code: code,
        data: data,
        last_modified: headers,
    };
}

#[derive(Deserialize, Debug, Clone)]
struct WinLossRecord {
    losses: usize,
    ties: usize,
    wins: usize,
}

impl WinLossRecord {
    fn new() -> WinLossRecord {
        WinLossRecord {
            losses: 0,
            ties: 0,
            wins: 0,
        }
    }
    
    fn to_usize(&self) -> usize {
        return 2 * self.wins + self.ties;
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TeamEventRanking {
    pub matches_played: usize,
    extra_stats: Vec<usize>,
    record: WinLossRecord,
    team_key: String,
}

#[derive(Deserialize, Queryable, Debug, Clone)]
pub struct RankingResultJSON {
    pub rankings: Vec<TeamEventRanking>,
}

impl TeamEventRanking {
    pub fn new(key: &str) -> TeamEventRanking {
        let mut new = TeamEventRanking {
            matches_played: 0,
            extra_stats: Vec::new(),
            record: WinLossRecord::new(),
            team_key: String::from(key),
        };
        new.extra_stats.push(0);
        return new;
    }
    
    pub fn to_usize(&mut self) -> usize {
        if self.extra_stats.len() == 0 {
            self.extra_stats.push(self.record.to_usize());
        }
        return self.extra_stats[0];
    }

    pub fn extra_prob(&mut self) -> f64 {
        let prob = (self.to_usize() - self.record.to_usize()) as f64 /
            self.matches_played as f64;
        if prob > 0f64 {
            return prob;
        }
        return 0f64;
    }

    pub fn add_win(&mut self) {
        self.record.wins += 1;
        self.matches_played += 1;
        self.extra_stats[0] += 2;
    }

    pub fn add_loss(&mut self) {
        self.record.losses += 1;
        self.matches_played += 1;
    }

    pub fn add_extra(&mut self) {
        if self.extra_stats.len() > 0 {
            self.extra_stats[0] += 1;
        }
    }

    pub fn key(&self) -> String {
        return self.team_key.clone();
    }
}

pub fn get_rankings(key: &str) -> Option<RankingResultJSON> {
    let url = format!("event/{}/rankings", key);
    let response = request(&url, &String::new());
    if response.code != 200 {
        return None;
    }
    let data_str = str::from_utf8(&response.data)
        .expect("Could not load event rankings string");
    let _ : RankingResultJSON = match serde_json::from_str(&data_str) {
        Ok(m) => return Some(m),
        Err(e) => {
            println!("Error: {}", e.description());
            return None;
        },
    };
}
