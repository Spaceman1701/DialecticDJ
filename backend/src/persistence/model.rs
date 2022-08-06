use std::time::Duration;

use rspotify::Token;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

pub struct PlaySession {
    pub id: Uuid,
    pub name: String,
    pub token: Option<Token>,
}
