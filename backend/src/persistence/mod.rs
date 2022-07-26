use std::env;

use anyhow::Result;
use ddj_core::types::Track;
use rocket::futures::StreamExt;
use sqlx::{postgres::PgPoolOptions, query::Query, Pool, Postgres, Row};

use crate::model::TrackInfo;

mod macros;
mod queries;

macro_rules! create_table {
    ($query:expr, $executor:expr) => {{
        let res = sqlx::query($query).execute($executor).await;
        if let Err(e) = &res {
            eprintln!("failed to run {}: {}", $query, e);
        } else {
            println!("successfully ran {}", $query);
        }
        res
    }};
}

///Trait abstracting storage requirements for DDJ
#[rocket::async_trait]
pub trait PersistentStore {
    async fn create_tables(&self) -> Result<()>;
    async fn get_track_queue(&self, limit: u32) -> Result<()>;
}

///Literally a Box<dyn PersistentStore + Send + Sync>
/// Using this type allows the database implementation to be
/// swapped at runtime
pub type Store = Box<dyn PersistentStore + Send + Sync>;

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

        Ok(Box::new(Self { pool: conn_pool }))
    }
}

#[rocket::async_trait]
impl PersistentStore for PostgressDatabase {
    async fn create_tables(&self) -> Result<()> {
        create_table!(queries::CREATE_ARTIST_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TASK_TABLE, &self.pool)?;
        create_table!(queries::CREATE_PLAYED_TRACKS_TABLE, &self.pool)?;
        create_table!(queries::CREATE_TRACK_QUEUE_TABLE, &self.pool)?;
        create_table!(queries::CREATE_ALUBMS_TABLE, &self.pool)?;

        Ok(())
    }

    async fn get_track_queue(&self, limit: u32) -> Result<()> {
        let mut result = sqlx::query(queries::GET_NEXT_N_TRACKS)
            .bind(limit as i32)
            .fetch(&self.pool);

        let tracks = result.map(|row| {});

        Ok(())
    }
}
