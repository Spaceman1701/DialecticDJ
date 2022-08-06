use std::{
    collections::{HashMap, HashSet},
    env,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};

use rspotify::Token;
use sqlx::{
    postgres::{PgPoolOptions, PgRow},
    types::chrono::{self, DateTime, Utc},
    Database, Executor, Pool, Postgres, Row, Transaction,
};
use uuid::Uuid;

use crate::{
    authentication::scopes,
    persistence::model::{PlaySession, SpotifyAlbum},
};

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
    executor: Pool<Postgres>,
}

impl PostgressDatabase {
    async fn new<'c>() -> Result<Self> {
        let hostname = env::var("POSTGRES_HOST")?;
        let user = env::var("POSTGRES_USER")?;
        let password = env::var("POSTGRES_PASSWORD")?;

        let connection_str = format!("postgres://{user}:{password}@{hostname}/ddj");

        let conn_pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_str)
            .await?;

        Ok(Self {
            executor: conn_pool,
        })
    }

    pub async fn connect() -> Result<Store> {
        let db = Self::new().await?;
        Ok(Arc::new(db))
    }
}

#[rocket::async_trait]
impl PersistentStore for PostgressDatabase {
    async fn create_tables(&self) -> Result<()> {
        create_table!(queries::CREATE_SESSION_TABLE, &self.executor)?;
        create_table!(queries::CREATE_ALUBMS_TABLE, &self.executor)?;
        create_table!(queries::CREATE_ARTIST_TABLE, &self.executor)?;
        create_table!(queries::CREATE_TRACKS_TABLE, &self.executor)?;
        create_table!(queries::CREATE_PLAYED_TRACKS_TABLE, &self.executor)?;
        create_table!(queries::CREATE_TRACK_QUEUE_TABLE, &self.executor)?;
        create_table!(queries::CREATE_ARTIST_TO_TRACK_TABLE, &self.executor)?;

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
            .fetch_all(&self.executor)
            .await?;

        result
            .into_iter()
            .map(|row| extract_track_from_row(&row))
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

        let mut tx = self.executor.begin().await?;

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

    async fn get_track_by_id(&self, id: &str) -> Result<SpotifyTrack> {
        const QUERY: &str = "
            SELECT 
                tracks.track_id         AS track_id, 
                tracks.name             AS track_name, 
                tracks.duration         AS track_dur, 
                tracks.album_id         AS album_id, 
                albums.name             AS album_name, 
                albums.cover_image_url  AS album_image 
            FROM tracks
            LEFT JOIN albums ON track.album_id = album.id
            WHERE tracks.track_id = $1
            LIMIT 1;
        ";

        let result = sqlx::query(QUERY)
            .bind(id)
            .fetch_one(&self.executor)
            .await?;

        extract_track_from_row(&result)
    }

    async fn pop_track_from_queue(&self) -> Result<Option<SpotifyTrack>> {
        const GET_NEXT_TRACK_QUERY: &str = "
            SELECT id FROM queued_tracks ORDER BY added_date DESC LIMIT 1;
        ";
        const REMOVE_AND_RETURN_QUERY: &str = "DELETE FROM queued_tracks WHERE id = $1;";

        let mut tx = self.executor.begin().await?;

        let result = sqlx::query(GET_NEXT_TRACK_QUERY)
            .fetch_optional(&mut tx)
            .await?;

        if let None = result {
            return Ok(None);
        }
        let unwrapped_res = result.unwrap();
        let target_id: &str = unwrapped_res.try_get("id")?;

        sqlx::query(REMOVE_AND_RETURN_QUERY)
            .bind(target_id)
            .execute(&mut tx)
            .await?;

        tx.commit().await?;

        let track = self.get_track_by_id(target_id).await;
        match track {
            Ok(t) => Ok(Some(t)),
            Err(e) => Err(anyhow::Error::msg(format!(
                "failed to retrieve track from db: {}",
                e
            ))),
        }
    }

    async fn create_session(&self, name: &str) -> Result<super::model::PlaySession> {
        const QUERY: &str = "
            INSERT INTO sessions (id, name, access_token, refresh_token, expires_at)
                VALUES ($1, $2, '', '', current_timestamp);
        ";
        let uuid = Uuid::new_v4();
        sqlx::query(QUERY)
            .bind(&uuid)
            .bind(name)
            .execute(&self.executor)
            .await?;

        Ok(PlaySession {
            id: uuid,
            name: name.to_owned(),
            token: None,
        })
    }

    async fn update_session(&self, session: &super::model::PlaySession) -> Result<()> {
        const QUERY: &str = "
            UPDATE sessions 
            SET name = $1, access_token = $2, refresh_token = $3, expires_at = $4
            WHERE id = $5;
        ";

        if let None = session.token {
            return Err(anyhow::Error::msg(
                "can't update database with unauthenticated session",
            ));
        }
        let token = session.token.as_ref().unwrap();
        let refresh_token = if token.refresh_token.is_none() {
            ""
        } else {
            token.refresh_token.as_ref().unwrap()
        };

        let expires_at = token.expires_at.as_ref();

        sqlx::query(QUERY)
            .bind(&session.name)
            .bind(&token.access_token)
            .bind(refresh_token)
            .bind(expires_at)
            .execute(&self.executor)
            .await?;

        Ok(())
    }

    async fn get_session(&self, id: Uuid) -> Result<Option<PlaySession>> {
        const QUERY: &str = "
            SELECT * FROM sessions WHERE id=$1;
        ";

        let maybe_row = sqlx::query(QUERY)
            .bind(&id)
            .fetch_optional(&self.executor)
            .await?;

        let res = maybe_row.map(|row| -> Result<PlaySession> {
            let name: String = row.try_get("name")?;
            let access_token: String = row.try_get("access_token")?;
            let refresh_token: String = row.try_get("refresh_token")?;
            let expires_at: DateTime<Utc> = row.try_get("expires_at")?;

            if access_token == "" {
                return Ok(PlaySession {
                    id,
                    name,
                    token: None,
                });
            }

            let rt_option = if refresh_token == "" {
                None
            } else {
                Some(refresh_token)
            };

            let mut token = Token::default();
            token.access_token = access_token;
            token.expires_at = Some(expires_at);
            token.refresh_token = rt_option;
            token.scopes = scopes();

            Ok(PlaySession {
                id,
                name,
                token: Some(token),
            })
        });

        todo!()
    }
}

fn extract_track_from_row(row: &PgRow) -> Result<SpotifyTrack> {
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

    pub const CREATE_SESSION_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS sessions (
        id uuid PRIMARY KEY, 
        name text,
        access_token text,
        refresh_token text,
        expires_at timestamp
    );
";
}

#[cfg(test)]
mod tests {

    use super::*;

    async fn setup_db<'c>() -> PostgressDatabase {
        let db = PostgressDatabase::new().await.unwrap();
        db.create_tables().await.unwrap();
        db
    }

    async fn teardown_tb<'c>(db: PostgressDatabase) {
        const DROP_SCHEMA: &str = "DROP SCHEMA public CASCADE;";
        const RECREATE_SCHEMA: &str = "CREATE SCHEMA public;";
        const GRANT_TO_PUBLIC: &str = "GRANT ALL ON SCHEMA public TO public;";

        sqlx::query(DROP_SCHEMA)
            .execute(&db.executor)
            .await
            .unwrap();
        sqlx::query(RECREATE_SCHEMA)
            .execute(&db.executor)
            .await
            .unwrap();

        sqlx::query(GRANT_TO_PUBLIC)
            .execute(&db.executor)
            .await
            .unwrap();
    }

    // async fn db_test(test: impl Future<Output = Result<()>>) -> Result<()> {
    //     let db = setup_db().await;
    //     let test_result = test.await; //TODO: handle panics
    //     teardown_tb(db).await;
    //     test_result
    // }

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
