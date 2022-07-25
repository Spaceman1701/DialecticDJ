pub const CREATE_TASK_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS tracks (
        name varchar,
        id varchar UNIQUE NOT NULL PRIMARY KEY,
    )
";

pub const CREATE_PLAYED_TRACKS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS played_tracks (
        id UNIQUE NOT NULL PRIMARY KEY,
        played_at TIMESTAMP,
        track_id REFERENCES tracks (id)
    )
";
