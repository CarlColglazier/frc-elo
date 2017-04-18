use super::models::Matche;
use std::collections::HashMap;
use probability::prelude::*;
use probability::distribution::Gaussian;
use super::CURRENT_YEAR;

const START_SCORE: f64 = 0f64;
const NEW_AVG: f64 = 150f64;
const SCORE_STD: &'static [f64] = &[17.6, 50.9, 45.6, 24.6, 28.4, 46.2,
    24.4, 21.0, 2.7, 28.4, 15.5, 31.1, 49.3, 33.2, 47.0, 95.0];

pub struct Teams {
    pub table: HashMap<String, f64>,
    k: f64,
    pub wins_correct: usize,
    carry_over: f64,
    pub brier: f64,
    pub total: usize,
    start_year: usize,
    current_year: usize,
    pub active_teams: [bool; 10000],
}

impl Teams {
    pub fn new(k: f64, carry_over: f64, start_year: usize) -> Teams {
        Teams {
            table: HashMap::new(),
            k: k,
            wins_correct: 0,
            carry_over: carry_over,
            brier: 0.0f64,
            total: 0,
            start_year: start_year,
            current_year: start_year,
            active_teams: [false; 10000],
        }
    }

    pub fn new_year(&mut self) {
        for (_, val) in self.table.iter_mut() {
            *val = *val * self.carry_over + NEW_AVG * (1f64 - self.carry_over);
        }
        self.current_year += 1;
    }

    pub fn get(&mut self, team: &String) -> f64 {
        let entry = self.table.entry(team.to_owned()).or_insert(START_SCORE);
        return *entry;
    }

    pub fn update(&mut self, team: &String, change: f64) {
        
        let mut entry = self.table.entry(team.to_owned()).or_insert(START_SCORE);
        *entry += change;
        if self.current_year == CURRENT_YEAR as usize {
            self.active_teams[team.replace("frc", "").parse::<usize>().unwrap()] = true;
        }
    }

    pub fn process_match(&mut self, m: &Matche) {
        let m = m.clone();
        let mut red = self.get(&m.red1) + self.get(&m.red2); //+ self.get(m.red3);
        if let Some(ref r) =  m.red3 {
            red += self.get(r);
        }
        let mut blue = self.get(&m.blue1) + self.get(&m.blue2);// + self.get(m.blue3);
        if let Some(ref r) = m.blue3 {
            blue += self.get(r);
        }
        let expected_r = 1f64 / (1f64 + 10f64.powf((blue - red) / 400f64));
        let actual_r = m.actual_r();
        let modifier;
        if m.comp_level == "qm" {
            modifier = 1f64;
        } else {
            modifier = 3f64;
        }
        let distribution = Gaussian::new(0.0, SCORE_STD[self.current_year - self.start_year]);
        let predicted_score_diff = distribution.inverse(expected_r);
        let score_margin_adj = (m.score_margin() as f64 - predicted_score_diff)
            / SCORE_STD[self.current_year - self.start_year];
        //let score_change;
        let change_r = self.k * score_margin_adj / modifier;
        self.update(&m.red1, change_r);
        self.update(&m.red2, change_r);
        if let Some(ref m) = m.red3 {
            self.update(m, change_r);
        };
        self.update(&m.blue1, -change_r);
        self.update(&m.blue2, -change_r);
        match m.blue3 {
            Some(ref m) => self.update(m, -change_r),
            None => {},
        };
        /*
        let team = "frc2642";
        if m.red1 == team || m.red2 == team || m.red3 == Some(String::from(team)) {
                println!("{}: E({:.0}) A({}) C({:.1}) N({:.1})", m.id, predicted_score_diff, m.score_margin(),
                         change_r, self.get(&String::from(team)));
        }
        if m.blue1 == team || m.blue2 == team || m.blue3 == Some(String::from(team)) {
            println!("{}: E({:.0}) A({}) C({:.1}) N({:.1})", m.id, -predicted_score_diff, -m.score_margin(),
                     -change_r, self.get(&String::from(team)));
        }*/
        // Accuracy measurement.
        // TODO: Allow this to be enabled using a flag.
        //if m.comp_level != "qm" &&
        //if m.id.contains("2012") || m.id.contains("2013") || m.id.contains("2014") {
        if m.id.contains("2017") {
            if m.actual_r() > 0.4 && m.actual_r() < 0.6 {
                return;
            }
            self.brier += (expected_r - actual_r).powf(2.0f64);
            self.total += 1;
            //println!("{},{}", m.red_score - m.blue_score, expected_r);
            //
            if (m.actual_r() - expected_r).abs() < 0.5f64 {
                self.wins_correct += 1;
            }
        }
    }
}
