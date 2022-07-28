use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyTrack {
    pub id: String,
    pub name: String,
    pub duration: Duration,
    pub album: SpotifyAlbum,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpotifyAlbum {
    pub name: String,
    pub id: String,
    pub cover_image_url: String,
}
