use std::{env, sync::Arc, time::Duration};

use anyhow::Result;
use ddj_core::types::Track;
use rocket::futures::StreamExt;
use sqlx::{postgres::PgPoolOptions, query::Query, Pool, Postgres, Row};

use crate::{model::TrackInfo, persistence::model::SpotifyAlbum};

use self::{model::SpotifyTrack, retrievers::QueuedTrackRef};

pub mod model;
pub mod pgsql;
mod retrievers;

///Trait abstracting storage requirements for DDJ
#[rocket::async_trait]
pub trait PersistentStore {
    async fn create_tables(&self) -> Result<()>;
    async fn get_track_queue(&self, limit: u32) -> Result<Vec<SpotifyTrack>>;
    async fn add_track_to_queue(&self, track: SpotifyTrack) -> Result<()>;
}

///Literally a Box<dyn PersistentStore + Send + Sync>
/// Using this type allows the database implementation to be
/// swapped at runtime
pub type Store = Arc<dyn PersistentStore + Send + Sync>;
