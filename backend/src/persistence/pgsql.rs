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
    async fn new() -> Result<Self> {
        let hostname = env::var("POSTGRES_HOST")?;
        let user = env::var("POSTGRES_USER")?;
        let password = env::var("POSTGRES_PASSWORD")?;

        let connection_str = format!("postgres://{user}:{password}@{hostname}/ddj");

        let conn_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await?;

        Ok(Self { pool: conn_pool })
    }
    pub async fn connect() -> Result<Store> {
        let db = Self::new().await?;
        Ok(Arc::new(db))
    }
}

#[rocket::async_trait]
impl PersistentStore for PostgressDatabase {
    async fn create_tables(&self) -> Result<()> {
        create_table!(queries::CREATE_ALUBMS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_ARTIST_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TRACKS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_PLAYED_TRACKS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TRACK_QUEUE_TABLE, &self.pool)?;
        create_table!(queries::CREATE_ARTIST_TO_TRACK_TABLE, &self.pool);

        Ok(())
    }

    async fn get_track_queue(&self, limit: u32) -> Result<Vec<SpotifyTrack>> {
        const QUERY: &str = "
            SELECT
                queued_tracks.track_id  AS track_id, 
                tracks.name             AS track_name, 
                tracks.duration         AS track_dur, 
                tracks.album_id         AS album_id, 
                albums.name             AS album_name, 
                albums.cover_image_url  AS album_image
            FROM queued_tracks
            LEFT JOIN tracks ON queued_tracks.track_id = tracks.id
            LEFT JOIN albums ON tracks.album_id = albums.id
            ORDER BY queued_tracks.added_date DESC 
            LIMIT ($1);";

        let result = sqlx::query(QUERY)
            .bind(limit as i32)
            .fetch_all(&self.pool)
            .await?;

        result
            .into_iter()
            .map(|row| -> Result<SpotifyTrack> {
                let track_id = row.try_get("track_id")?;
                let track_name = row.try_get("track_name")?;
                let track_duration: i64 = row.try_get("track_dur")?;

                let album_id = row.try_get("album_id")?;
                let album_name = row.try_get("album_name")?;
                let album_cover_image_url = row.try_get("album_image")?;

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
            INSERT INTO tracks (id, name, album_id, duration)
                VALUES ($1, $2, $3, $4)
            ON CONFLICT DO NOTHING;
        ";
        const INSERT_ALBUM_QUERY: &str = "
            INSERT INTO albums (id, name, cover_image_url)
                VALUES ($1, $2, $2)
            ON CONFLICT DO NOTHING;
        ";
        const INSERT_QUEUED_QUERY: &str = "
            INSERT INTO queued_tracks (track_id)
                VALUES ($1)
        ";

        let mut tx = self.pool.begin().await?;

        sqlx::query(INSERT_ALBUM_QUERY)
            .bind(&track.album.id)
            .bind(&track.album.name)
            .bind(&track.album.cover_image_url)
            .execute(&mut tx)
            .await?;

        sqlx::query(INSERT_TRACK_QUERY)
            .bind(&track.id)
            .bind(&track.name)
            .bind(&track.album.id)
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

mod queries {
    pub const CREATE_TRACKS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS tracks (
        name text,
        id text PRIMARY KEY,
        album_id text REFERENCES albums (id),
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
        added_date timestamp DEFAULT current_timestamp,
        track_id text REFERENCES tracks (id)
    );
";
}

#[cfg(test)]
mod tests {
    use core::panic;
    use std::{any::Any, future::Future};

    use super::*;

    async fn setup_db() -> PostgressDatabase {
        let db = PostgressDatabase::new().await.unwrap();
        db.create_tables().await.unwrap();
        db
    }

    async fn teardown_tb(db: PostgressDatabase) {
        const DROP_SCHEMA: &str = "DROP SCHEMA public CASCADE;";
        const RECREATE_SCHEMA: &str = "CREATE SCHEMA public;";
        const GRANT_TO_PUBLIC: &str = "GRANT ALL ON SCHEMA public TO public;";

        sqlx::query(DROP_SCHEMA).execute(&db.pool).await.unwrap();
        sqlx::query(RECREATE_SCHEMA)
            .execute(&db.pool)
            .await
            .unwrap();

        sqlx::query(GRANT_TO_PUBLIC)
            .execute(&db.pool)
            .await
            .unwrap();
    }

    async fn db_test(test: impl Future<Output = Result<()>>) -> Result<()> {
        let db = setup_db().await;
        let test_result = test.await; //TODO: handle panics
        teardown_tb(db).await;
        test_result
    }

    #[tokio::test]
    async fn test_add_track_to_queue() -> Result<()> {
        let db = setup_db().await;

        db.create_tables().await?;
        db.add_track_to_queue(SpotifyTrack {
            id: "abcde".to_owned(),
            name: "Example Song".to_owned(),
            duration: Duration::from_secs(360),
            album: SpotifyAlbum {
                name: "Example Album".to_owned(),
                id: "abcdefg".to_owned(),
                cover_image_url: "http://fake-album-cover.com/image.jpg".to_owned(),
            },
        })
        .await?;

        let tracks = db.get_track_queue(1).await?;
        assert_eq!(tracks.len(), 1);

        let retrieved = tracks.get(0).unwrap();
        assert_eq!(retrieved.name, "Example Song");

        teardown_tb(db).await;

        Ok(())
    }
}
