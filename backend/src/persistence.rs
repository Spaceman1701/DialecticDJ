use ddj_core::types::Track;
use std::{collections::VecDeque, time::Duration};

use rocket::{
    serde::{Deserialize, Serialize},
    tokio::sync::RwLock,
};
use rspotify::model::{FullAlbum, FullTrack, SimplifiedAlbum, TrackId};

pub struct DataStore {
    inner: RwLock<InnerDataStore>,
}

struct InnerDataStore {
    queue: VecDeque<TrackInfo>,
    current_track: Option<TrackInfo>,
}

impl Default for InnerDataStore {
    fn default() -> Self {
        Self {
            queue: Default::default(),
            current_track: Default::default(),
        }
    }
}

impl DataStore {
    pub fn new() -> DataStore {
        return DataStore {
            inner: Default::default(),
        };
    }

    pub async fn peek_first(&self) -> Option<TrackInfo> {
        let readable = self.inner.read().await;
        let front = readable.queue.front();

        return front.cloned();
    }

    pub async fn get_all_tracks(&self) -> Vec<TrackInfo> {
        let readable = self.inner.read().await;
        return readable.queue.iter().map(|info| info.clone()).collect();
    }

    pub async fn add_track(&self, track: TrackInfo) {
        let mut writable = self.inner.write().await;
        writable.queue.push_back(track);
    }

    pub async fn pop_first_track(&self) -> Option<TrackInfo> {
        let mut writable = self.inner.write().await;
        return writable.queue.pop_front();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: TrackId,
    pub name: String,
    pub duration: Duration,
    pub album: SimplifiedAlbum,
}

impl From<FullTrack> for TrackInfo {
    fn from(track: FullTrack) -> Self {
        return TrackInfo {
            id: track.id.unwrap(),
            name: track.name,
            duration: track.duration,
            album: track.album,
        };
    }
}

impl Into<Track> for &TrackInfo {
    fn into(self) -> Track {
        Track {
            name: self.name.clone(),
            id: self.id.to_string(),
            artists: Vec::new(),
            duration: self.duration,
            album_art_link: self.album.images.first().map(|image| image.url.clone()),
        }
    }
}
