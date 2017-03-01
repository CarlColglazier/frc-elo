PRAGMA foreign_keys = ON;

CREATE TABLE teams (
       team_numer PRIMARY KEY,
       nickname TEXT,
       key TEXT
);

CREATE TABLE events (
       id TEXT PRIMARY KEY NOT NULL,
       name TEXT NOT NULL,
       event_type INTEGER NOT NULL,
       official INTEGER NOT NULL,
       start_date TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS matches (
       id TEXT PRIMARY KEY NOT NULL,
       comp_level TEXT NOT NULL,
       match_number INTEGER NOT NULL,
       set_number INTEGER NOT NULL,
       event_id TEXT NOT NULL,
       red_score INTEGER NOT NULL,
       blue_score INTEGER NOT NULL,
       red1 TEXT NOT NULL,
       red2 TEXT NOT NULL,
       red3 TEXT,
       blue1 TEXT NOT NULL,
       blue2 TEXT NOT NULL,
       blue3 TEXT,
       FOREIGN KEY(event_id) REFERENCES events(id)
);
