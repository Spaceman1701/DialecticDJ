use std::sync::Arc;

use anyhow::Result;

use self::model::SpotifyTrack;

pub mod model;
pub mod pgsql;

///Trait abstracting storage requirements for DDJ
#[rocket::async_trait]
pub trait PersistentStore {
    async fn create_tables(&self) -> Result<()>;
    async fn get_track_queue(&self, limit: u32) -> Result<Vec<SpotifyTrack>>;
    async fn add_track_to_queue(&self, track: SpotifyTrack) -> Result<()>;
    async fn pop_track_from_queue(&self) -> Result<Option<SpotifyTrack>>;
    async fn get_track_by_id(&self, id: &str) -> Result<SpotifyTrack>;
}

///Literally a Box<dyn PersistentStore + Send + Sync>
/// Using this type allows the database implementation to be
/// swapped at runtime
pub type Store = Arc<dyn PersistentStore + Send + Sync>;
