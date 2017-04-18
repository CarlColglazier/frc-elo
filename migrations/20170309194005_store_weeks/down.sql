ALTER TABLE events RENAME TO temp_events;
 
CREATE TABLE events (
       id TEXT PRIMARY KEY NOT NULL,
       name TEXT NOT NULL,
       event_type INTEGER NOT NULL,
       official INTEGER NOT NULL,
       start_date TEXT NOT NULL
);

INSERT INTO events
SELECT id, name, event_type, official, start_date
FROM temp_events;
 
DROP TABLE temp_events;

