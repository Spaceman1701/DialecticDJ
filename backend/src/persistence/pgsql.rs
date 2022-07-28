use std::{env, ops::Add, sync::Arc, time::Duration};

use rocket::http::private::cookie::Display;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};

use crate::persistence::model::SpotifyAlbum;

use super::{model::SpotifyTrack, PersistentStore, Store};

use anyhow::Result;

macro_rules! create_table {
    ($query:expr, $executor:expr) => {{
        let res = sqlx::query($query).execute($executor).await;
        if let Err(e) = &res {
            eprintln!("failed to run {}: {}", $query, e);
        } else {
            println!("successfully ran {}", stringify!($query));
        }
        res
    }};
}

pub struct PostgressDatabase {
    pool: Pool<Postgres>,
}

impl PostgressDatabase {
    pub async fn connect() -> Result<Store> {
        let hostname = env::var("POSTGRES_HOST")?;
        let user = env::var("POSTGRES_USER")?;
        let password = env::var("POSTGRES_PASSWORD")?;

        let connection_str = format!("postgres://{user}:{password}@{hostname}/ddj");

        let conn_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await?;

        Ok(Arc::new(Self { pool: conn_pool }))
    }
}

#[rocket::async_trait]
impl PersistentStore for PostgressDatabase {
    async fn create_tables(&self) -> Result<()> {
        create_table!(queries::CREATE_ARTIST_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TRACKS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_PLAYED_TRACKS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TRACK_QUEUE_TABLE, &self.pool)?;
        create_table!(queries::CREATE_ALUBMS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_ARTIST_TO_TRACK_TABLE, &self.pool);

        Ok(())
    }

    async fn get_track_queue(&self, limit: u32) -> Result<Vec<SpotifyTrack>> {
        const QUERY: &str = "
            SELECT 
            (
                track_queue.id, 
                tracks.name, 
                tracks.duration, 
                albums.id, 
                albums.name, 
                albums.cover_image_url
            ) FROM track_queue
            LEFT JOIN track_queue.id = tracks.id
            LEFT JOIN tracks.album_id = albums.id
            ORDER BY tracks.added_date DESC 
            LIMIT ($1);";

        let result = sqlx::query(QUERY)
            .bind(limit as i32)
            .fetch_all(&self.pool)
            .await?;

        result
            .into_iter()
            .map(|row| -> Result<SpotifyTrack> {
                let track_id = row.try_get("track_queue.id")?;
                let track_name = row.try_get("tracks.name")?;
                let track_duration: i64 = row.try_get("tracks.duration")?;

                let album_id = row.try_get("albums.id")?;
                let album_name = row.try_get("albums.name")?;
                let album_cover_image_url = row.try_get("albums.cover_image_url")?;

                let track = SpotifyTrack {
                    id: track_id,
                    name: track_name,
                    duration: Duration::from_secs(track_duration as u64),
                    album: SpotifyAlbum {
                        id: album_id,
                        name: album_name,
                        cover_image_url: album_cover_image_url,
                    },
                };

                Ok(track)
            })
            .collect()
    }

    async fn add_track_to_queue(&self, track: SpotifyTrack) -> Result<()> {
        const INSERT_TRACK_QUERY: &str = "
            INSERT INTO tracks (id, name, duration)
                VALUES ($1, $2, $3)
            ON DO NOTHING;
        ";
        const INSERT_QUEUED_QUERY: &str = "
            INSERT INTO queued_tracks (track_id)
                VALUES ($1, $2)
        ";

        let mut tx = self.pool.begin().await?;

        sqlx::query(INSERT_TRACK_QUERY)
            .bind(&track.id)
            .bind(&track.name)
            .bind(track.duration.as_secs() as i64)
            .execute(&mut tx)
            .await?;

        sqlx::query(INSERT_QUEUED_QUERY)
            .bind(&track.id)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
}

struct Column(&'static str, &'static str, Option<&'static str>);

impl Column {
    fn get_descriptor(&self) -> String {
        if let Some(constraint) = self.2 {
            format!("{} {} {}", self.0, self.1, self.2.unwrap())
        } else {
            format!("{} {}", self.0, self.1)
        }
    }
}

mod queries {
    pub const CREATE_TRACKS_TABLE: &str = "
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
        track_id text REFERENCES tracks (id),
        artist_id text REFERENCES artists (id),
        PRIMARY KEY(track_id, artist_id)
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
        id SERIAL PRIMARY KEY,
        played_date TIMESTAMP,
        track_id text REFERENCES tracks (id)
    );
";

    pub const CREATE_TRACK_QUEUE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS queued_tracks (
        id SERIAL PRIMARY KEY,
        added_date TIMESTAMP,
        track_id text REFERENCES tracks (id)
    );
";
}
