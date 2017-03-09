use super::models::Matche;
use std::collections::HashMap;

const START_SCORE: f64 = 0f64;

pub struct Teams {
    pub table: HashMap<String, f64>,
    k: f64,
    carry_over: f64,
    pub brier: f64,
    pub total: usize,
}

impl Teams {
    pub fn new(k: f64, carry_over: f64) -> Teams {
        Teams {
            table: HashMap::new(),
            k: k,
            carry_over: carry_over,
            brier: 0.0f64,
            total: 0,
        }
    }

    pub fn new_year(&mut self) {
        for (_, val) in self.table.iter_mut() {
            *val *= self.carry_over;
        }
    }

    fn get(&mut self, team: &String) -> f64 {
        let entry = self.table.entry(team.to_owned()).or_insert(START_SCORE);
        return *entry;
    }

    pub fn update(&mut self, team: &String, change: f64) {
        let mut entry = self.table.entry(team.to_owned()).or_insert(START_SCORE);
        *entry += change;
    }

    pub fn process_match(&mut self, m: &Matche) {
        let m = m.clone();
        let red = self.get(&m.red1) + self.get(&m.red2); //+ self.get(m.red3);
        let blue = self.get(&m.blue1) + self.get(&m.blue2);// + self.get(m.blue3);
        let expected_r = 1f64 / (1f64 + 10f64.powf((blue - red) / 400f64));
        let actual_r;
        if m.red_score > m.blue_score {
            actual_r = 1.0f64;
        } else if m.red_score < m.blue_score {
            actual_r = 0.0f64;
        } else {
            actual_r = 0.5f64;
        }
        let change_r = self.k * (actual_r - expected_r);
        self.update(&m.red1, change_r);
        self.update(&m.red2, change_r);
        match m.red3 {
            Some(ref m) => self.update(m, change_r),
            None => {},
        };
        self.update(&m.blue1, -change_r);
        self.update(&m.blue2, -change_r);
        match m.blue3 {
            Some(ref m) => self.update(m, -change_r),
            None => {},
        };
        // Accuracy measurement.
        // TODO: Allow this to be enabled using a flag.
        //if m.comp_level != "qm" &&
        if m.id.contains("2017") {
            let actual;
            /*
            if m.red_score + m.blue_score == 0 {
                actual = 0.5f64;
            } else {
                actual = m.red_score as f64 / (m.red_score as f64 + m.blue_score as f64);
        }*/
            if m.red_score > m.blue_score {
                actual = 1.0f64;
            } else if m.red_score < m.blue_score {
                actual = 0.0f64;
            } else {
                actual = 0.5f64;
            }
            self.brier += (expected_r - actual).powf(2.0f64);
            self.total += 1;
            //println!("{},{}", m.red_score - m.blue_score, expected_r);
            //
        }
    }
}
