pub const CREATE_TASK_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS tracks (
        name text,
        id text PRIMARY KEY,
        duration bigint        
    );
";

pub const CREATE_ARTIST_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS artists (
        name text,
        id text PRIMARY KEY
    );
";

pub const CREATE_ARTIST_TO_TRACK_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS artist_to_track (
        track_id PRIMARY KEY REFERENCES tracks (id),
        artist_id PRIMARY KEY REFERENCES artists (id)
    );
";

pub const CREATE_ALUBMS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS albums (
        id text PRIMARY KEY,
        name text,
        cover_image_url text
    );
";

pub const CREATE_PLAYED_TRACKS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS played_tracks (
        id uuid PRIMARY KEY,
        played_date TIMESTAMP,
        track_id text REFERENCES tracks (id)
    );
";

pub const CREATE_TRACK_QUEUE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS track_queue (
        id uuid PRIMARY KEY,
        added_date TIMESTAMP,
        track_id text REFERENCES tracks (id)
    );
";

pub const GET_NEXT_N_TRACKS: &str = "
    SELECT * FROM track_queue ORDER BY added_date DESC LIMIT ($1);
";
