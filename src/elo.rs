use super::models::Matche;
use std::collections::HashMap;
use probability::prelude::*;
use probability::distribution::Gaussian;
use super::CURRENT_YEAR;

const START_SCORE: f64 = 0f64;
const NEW_AVG: f64 = 150f64;
const SCORE_STD: &'static [f64] = &[17.6, 50.9, 45.6, 24.6, 28.4, 46.2,
    24.4, 21.0, 2.7, 28.4, 15.5, 31.1, 49.3, 33.2, 47.0, 95.0];

#[derive(Clone)]
pub struct Teams {
    pub table: HashMap<String, f64>,
    k: f64,
    pub wins_correct: usize,
    carry_over: f64,
    pub brier: f64,
    pub total: usize,
    start_year: usize,
    current_year: usize,
    pub active_teams: Vec<bool>,
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
            active_teams: vec![false; 10000],
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

    pub fn sum_elo(&mut self, m: &Matche, red: bool) -> f64 {
        let mut score;
        if red {
            score = self.get(&m.red1) + self.get(&m.red2);
            if let Some(ref t) = m.red3 {
                score += self.get(t);
            }
        } else {
            score = self.get(&m.blue1) + self.get(&m.blue2);
            if let Some(ref t) = m.blue3 {
                score += self.get(t);
            }
        }
        return score;
    }

    pub fn predict(&mut self, m: &Matche) -> f64 {
        let m = m.clone();
        let red = self.sum_elo(&m, true);
        let blue = self.sum_elo(&m, false);
        return 1f64 / (1f64 + 10f64.powf((blue - red) / 400f64));
    }

    pub fn predict_diff(&self, expected: f64) -> f64 {
        let distribution = Gaussian::new(0.0, SCORE_STD[self.current_year - self.start_year]);
        return distribution.inverse(expected);
    }

    pub fn process_match(&mut self, m: &Matche) {
        let m = m.clone();
        let expected_r = self.predict(&m);
        let actual_r = m.actual_r();
        let modifier;
        if m.comp_level == "qm" {
            modifier = 1f64;
        } else {
            modifier = 3f64;
        }
        let predicted_score_diff = self.predict_diff(expected_r);
        let score_margin_adj = (m.score_margin() as f64 - predicted_score_diff)
            / SCORE_STD[self.current_year - self.start_year];
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
        // TODO: Allow this to be enabled using a flag.
        //if m.comp_level != "qm" &&
        //if m.id.contains("2012") || m.id.contains("2013") || m.id.contains("2014") {
        if m.id.contains("2017") {
            if m.actual_r() > 0.4 && m.actual_r() < 0.6 {
                return;
            }
            self.brier += (expected_r - actual_r).powf(2.0f64);
            self.total += 1;
            if (m.actual_r() - expected_r).abs() < 0.5f64 {
                self.wins_correct += 1;
            }
        }
    }

    pub fn simulate(&mut self, m: &Matche) -> bool {
        let mut m = m.clone();
        let expected_r = self.predict(&m);
        let predicted_score_diff = self.predict_diff(expected_r);
        let distribution = Gaussian::new(predicted_score_diff,
                                         SCORE_STD[self.current_year - self.start_year]);
        let mut source = source::default();
        // Actual is the actual score.
        let actual = distribution.sample(&mut source);
        m.red_score = actual as i32;
        m.blue_score = 0;
        self.process_match(&m);
        return actual > 0.0f64;
    }
}
