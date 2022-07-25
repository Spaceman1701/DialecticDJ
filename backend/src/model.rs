use ddj_core::types::Track;
use std::time::Duration;

use rocket::serde::{Deserialize, Serialize};
use rspotify::model::{FullTrack, SimplifiedAlbum, TrackId};

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
