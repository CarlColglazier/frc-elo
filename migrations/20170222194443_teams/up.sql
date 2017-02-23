PRAGMA foreign_keys = ON;

CREATE TABLE teams (
       team_numer PRIMARY KEY,
       nickname TEXT,
       key TEXT
);

CREATE TABLE events (
       key TEXT PRIMARY KEY,
       name TEXT,
       event_type INTEGER,
       official INTEGER,
       start_date TEXT
);

CREATE TABLE IF NOT EXISTS matches (
       key TEXT PRIMARY KEY,
       comp_level TEXT,
       match_number INTEGER,
       set_number INTEGER,
       event_key TEXT,
       red_score INTEGER,
       blue_score INTEGER,
       red1 TEXT,
       red2 TEXT,
       red3 TEXT,
       blue1 TEXT,
       blue2 TEXT,
       blue3 TEXT,
       FOREIGN KEY(event_key) REFERENCES events(event_key)
);
