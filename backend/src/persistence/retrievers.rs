use crate::model::{Album, TrackInfo};

use super::Store;

#[rocket::async_trait]
pub trait Retrieve<T> {
    async fn retrieve(&self, store: &Store) -> T;
}

pub struct QueuedTrackRef {
    pub added_time: u64,
    pub track_id: String,
}

#[rocket::async_trait]
impl Retrieve<TrackInfo> for QueuedTrackRef {
    async fn retrieve(&self, store: &Store) -> TrackInfo {
        todo!()
    }
}

#[rocket::async_trait]
impl Retrieve<Vec<TrackInfo>> for Vec<QueuedTrackRef> {
    async fn retrieve(&self, store: &Store) -> Vec<TrackInfo> {
        todo!()
    }
}

pub struct AlbumRef {
    pub album_id: String,
}

#[rocket::async_trait]
impl Retrieve<Album> for AlbumRef {
    async fn retrieve(&self, store: &Store) -> Album {
        todo!()
    }
}
