use super::models::Matche;
use std::collections::HashMap;
use std::f64::consts::PI;

const START_RATING: f64 = 0f64; //1500f64;
const START_RD: f64 = 350f64;
const Q: f64 = 0.01151292546;
const C: f64 = 30f64;
const GAP_WEEKS: f64 = 12f64;

#[derive(Clone, Debug)]
pub struct Glicko {
    pub rating: f64,
    pub deviation: f64,
}
fn g(rd: f64) -> f64 {
    return 1f64 /
        (1f64 + (3f64 * Q.powf(2f64) * rd.powf(2f64)) /
         PI.powf(2f64)).sqrt();
}

fn e(a_r: f64, r_j: f64, rd_j: f64) -> f64 {
    return 1f64 /
        (1f64 + 10f64.powf(-g(rd_j) * (a_r - r_j) / 400f64));
}


impl Glicko {
    pub fn new() -> Glicko {
        Glicko {
            rating: START_RATING,
            deviation: START_RD,
        }
    }

    pub fn predict(&self, other: &Glicko) -> f64 {
        return 1f64 /
            (1f64 + 10f64.powf(-g((self.deviation.powf(2f64) + other.deviation.powf(2f64)).sqrt())
                               * (self.rating - other.rating) / 200f64));
    }
}

#[derive(Clone, Debug)]
pub struct GlickoTeam {
    pub glicko: Glicko,
    pub results: Vec<f64>,
    pub opponents: Vec<Glicko>,
    pub last_week: i32,
}

impl GlickoTeam {
    pub fn new() -> GlickoTeam {
        GlickoTeam {
            glicko: Glicko::new(),
            results: Vec::new(),
            opponents: Vec::new(),
            last_week: 0,
        }
    }

    pub fn soft_process(&self) -> Glicko {
        let mut r_sum = 0f64;
        let mut d_square = 0f64;
        for i in 0..self.results.len() {
            let result = self.results[i];
            let ref opponent = self.opponents[i];
            let er = e(self.glicko.rating, opponent.rating, opponent.deviation);
            let g = g(opponent.deviation);
            d_square += g.powf(2f64) * er * (1f64 - er);
            r_sum += g * (result - er);
        }
        d_square *= Q.powf(2f64);
        d_square = d_square.powf(-1f64);
        let rating = self.glicko.rating + r_sum * Q /
            ((1f64 / self.glicko.deviation.powf(2f64)) + (1f64 / d_square));
        let deviation = ((1f64 / self.glicko.deviation.powf(2f64)) + (1f64 / d_square))
            .powf(-1f64).sqrt();
        return Glicko {
            rating: rating,
            deviation: deviation,
        };
    }

    pub fn process(&mut self) {
        let mut r_sum = 0f64;
        let mut d_square = 0f64;
        for i in 0..self.results.len() {
            let result = self.results[i];
            let ref opponent = self.opponents[i];
            let er = e(self.glicko.rating, opponent.rating, opponent.deviation);
            let g = g(opponent.deviation);
            d_square += g.powf(2f64) * er * (1f64 - er);
            r_sum += g * (result - er);
        }
        d_square *= Q.powf(2f64);
        d_square = d_square.powf(-1f64);
        self.glicko.rating += r_sum * Q /
            ((1f64 / self.glicko.deviation.powf(2f64)) + (1f64 / d_square));
        self.glicko.deviation = ((1f64 / self.glicko.deviation.powf(2f64)) + (1f64 / d_square))
            .powf(-1f64).sqrt();
        self.results.clear();
        self.opponents.clear();
    }
}
#[derive(Clone, Debug)]
pub struct GlickoTeams {
    pub table: HashMap<String, GlickoTeam>,
    pub wins_correct: usize,
    pub brier: f64,
    pub total: usize,
}

impl GlickoTeams {
    pub fn new() -> GlickoTeams {
        GlickoTeams {
            table: HashMap::new(),
            wins_correct: 0,
            brier: 0f64,
            total: 0,
        }
    }

    pub fn start_event(&mut self, week: i32) {
        for (_, val) in self.table.iter_mut() {
            let time = week - val.last_week;
            if time <= 0 {
                continue;
            }
            val.last_week = week;
            val.glicko.deviation = (val.glicko.deviation.powf(2f64) + time as f64 * C.powf(2f64))
                .sqrt();
            if val.glicko.deviation > START_RD {
                val.glicko.deviation = START_RD;
            }
        }
    }

    pub fn finish_event(&mut self) {
        for (_, val) in self.table.iter_mut() {
            val.process();
        }
    }

    pub fn new_year(&mut self) {
        for (_, val) in self.table.iter_mut() {
            val.last_week = 0;
            //val.glicko.rating = val.glicko.rating * 0.9f64 + 400f64 * 0.1;
            val.glicko.deviation = (val.glicko.deviation.powf(2f64) + GAP_WEEKS * C.powf(2f64))
                .sqrt();
            if val.glicko.deviation > START_RD {
                val.glicko.deviation = START_RD;
            }
        }
    }

    pub fn get_team(&mut self, team: &String) -> &mut GlickoTeam {
        let entry = self.table.entry(team.to_owned()).or_insert(GlickoTeam::new());
        return entry;
    }

    fn get(&mut self, team: &String) -> Glicko {
        let entry = self.table.entry(team.to_owned()).or_insert(GlickoTeam::new());
        return entry.soft_process();
    }

    pub fn average(&mut self, teams: &Vec<String>) -> Glicko {
        let mut rating = 0f64;
        let mut deviation = 0f64;
        for team in teams {
            let val = self.get(&team);
            rating += val.rating;
            deviation += val.deviation;
        }
        Glicko {
            rating: rating / teams.len() as f64,
            deviation: deviation / teams.len() as f64,
        }
    }

    pub fn process_match(&mut self, m: &Matche) {
        let m = m.clone();
        let red = m.get_red();
        let blue = m.get_blue();
        let red_glicko = self.average(&red);
        let blue_glicko = self.average(&blue);
        
        for team in &red {
            let mut record = self.get_team(team);
            record.results.push(m.actual_r());
            record.opponents.push(blue_glicko.clone());
        }
        for team in &blue {
            let mut record = self.get_team(team);
            record.results.push(m.actual_b());
            record.opponents.push(red_glicko.clone());
        }
        //if m.id.contains("2012") || m.id.contains("2013") || m.id.contains("2014") {
        if m.id.contains("2017") {
            if m.actual_r() > 0.4 && m.actual_r() < 0.6 {
                // This is a tie
                return;
            }
            let predicted = red_glicko.predict(&blue_glicko);
            let mid = 0.85f64;
            let range = 0.15f64;
            if predicted > mid + range || predicted < (1f64 - mid) - range ||
                (predicted < mid - range && predicted > (1f64 - mid) + range) {
                return;
            }
            self.brier += (predicted - m.actual_r()).powf(2.0f64);
            self.total += 1;
            if (m.actual_r() - predicted).abs() < 0.5f64 {
                self.wins_correct += 1;
            }
        }
    }
}
