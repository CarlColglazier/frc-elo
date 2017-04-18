use super::schema::*;

#[derive(Deserialize, Queryable, Debug, Clone)]
pub struct EventJSON {
    pub key: String,
    pub name: String,
    pub event_type: usize,
    //pub official: Option<bool>,
    pub start_date: String,
    pub week: Option<i32>,
}

#[derive(Queryable, Identifiable, Associations)]
#[has_many(matches)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub event_type: i32,
    pub official: i32,
    pub start_date: String,
    pub week: i32,
}

#[derive(Insertable)]
#[table_name="events"]
pub struct NewEvent<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub event_type: i32,
    pub official: i32,
    pub start_date: &'a str,
    pub week: i32,
}

pub fn prepare_event(event: &EventJSON) -> NewEvent {
    NewEvent {
        id: &event.key,
        name: &event.name,
        event_type: event.event_type as i32,
        official: match event.event_type {
            99 => 0,
            _ => 1,
        },
        week: match event.week {
            Some(w) => w,
            None => 7,
        },
        start_date: &event.start_date,
    }
}

#[derive(Deserialize, Queryable, Debug, Clone)]
pub struct Alliances {
    pub red: Alliance,
    pub blue: Alliance,
}

#[derive(Deserialize, Queryable, Debug, Clone)]
pub struct Alliance {
    pub score: i32,
    pub team_keys: Vec<String>
}

#[derive(Deserialize, Queryable, Debug, Clone)]
pub struct GameMatch {
    pub key: String,
    pub comp_level: String,
    pub match_number: i32,
    pub set_number: i32,
    pub event_key: String,
    pub alliances: Alliances,
}

#[derive(Debug, Queryable, Identifiable, Associations)]
#[belongs_to(Event)]
pub struct Matche {
    pub id: String,
    pub comp_level: String,
    pub match_number: i32,
    pub set_number: i32,
    pub event_id: String,
    pub red_score: i32,
    pub blue_score: i32,
    pub red1: String,
    pub red2: String,
    pub red3: Option<String>,
    pub blue1: String,
    pub blue2: String,
    pub blue3: Option<String>,
}

impl Matche {
    pub fn get_red(&self) -> Vec<String> {
        let mut teams = vec!(self.red1.clone(), self.red2.clone());
        if let Some(ref red3) = self.red3 {
            teams.push(red3.to_owned());
        }
        return teams;
    }

    pub fn get_blue(&self) -> Vec<String> {
        let mut teams = vec!(self.blue1.clone(), self.blue2.clone());
        if let Some(ref blue3) = self.blue3 {
            teams.push(blue3.to_owned());
        }
        return teams;
    }

    pub fn actual_r(&self) -> f64 {
        if self.red_score > self.blue_score {
            return 1.0f64;
        } else if self.red_score < self.blue_score {
            return 0.0f64;
        }
        return 0.5f64;
    }

    pub fn actual_b(&self) -> f64 {
        if self.red_score > self.blue_score {
            return 0.0f64;
        } else if self.red_score < self.blue_score {
            return 1.0f64;
        }
        return 0.5f64;
    }

    pub fn score_margin(&self) -> i32 {
        return self.red_score - self.blue_score;
    }
}

#[derive(Insertable)]
#[table_name="matches"]
pub struct NewMatch<'a> {
    pub id: &'a str,
    pub comp_level: &'a str,
    pub match_number: i32,
    pub set_number: i32,
    pub event_id: &'a str,
    pub red_score: i32,
    pub blue_score: i32,
    pub red1: &'a str,
    pub red2: &'a str,
    pub red3: Option<&'a str>,
    pub blue1: &'a str,
    pub blue2: &'a str,
    pub blue3: Option<&'a str>,
}



pub fn prepare_match(game_match: &GameMatch) -> Option<NewMatch> {
    Some(NewMatch {
        id: &game_match.key,
        comp_level: &game_match.comp_level,
        match_number: game_match.match_number,
        set_number: game_match.set_number,
        event_id: &game_match.event_key,
        red_score: game_match.alliances.red.score,
        blue_score: game_match.alliances.blue.score,
        red1: match game_match.alliances.red.team_keys.get(0) {
            Some(i) => i,
            None => return None,
        },
        red2: match game_match.alliances.red.team_keys.get(1) {
            Some(i) => i,
            None => return None,
        },
        red3: match game_match.alliances.red.team_keys.get(2) {
            Some(i) => Some(i),
            None => None,
        },
        blue1: match game_match.alliances.blue.team_keys.get(0) {
            Some(i) => i,
            None => return None,
        },
        blue2: match game_match.alliances.blue.team_keys.get(1) {
            Some(i) => i,
            None => return None,
        },
        blue3: match game_match.alliances.blue.team_keys.get(2) {
            Some(i) => Some(i),
            None => None,
        },
    })
}

/*
#[derive(Deserialize, Queryable)]
pub struct Team {
team_number: u16,
nickname: String,
key: String,
}
*/
