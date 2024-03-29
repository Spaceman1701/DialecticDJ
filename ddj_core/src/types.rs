use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artist {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Track {
    pub name: String,
    pub artists: Vec<Artist>,
    pub duration: Duration,
    pub id: String,
    pub album_art_link: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerState {
    pub current_track: Option<Track>,
    pub queue: Vec<Track>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Session {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateSessionResponse {
    pub session: Session,
    pub auth_link: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthenticateClientMessage {
    pub session_id: Uuid,
    pub auth_code: String,
}

#[cfg(feature = "rspotify")]
pub mod conversions {
    use crate::types::Track;
    use rspotify::model::FullTrack;
    use rspotify::model::Id;
    use rspotify::model::SimplifiedArtist;

    use crate::types::Artist;

    impl From<&FullTrack> for Track {
        fn from(full_track: &FullTrack) -> Self {
            Self {
                name: full_track.name.clone(),
                artists: full_track.artists.iter().map(|a| a.into()).collect(),
                duration: full_track.duration,
                id: full_track.id.as_ref().unwrap().id().to_owned(),
                album_art_link: full_track
                    .album
                    .images
                    .first()
                    .map(|image| image.url.clone()),
            }
        }
    }

    impl From<&SimplifiedArtist> for Artist {
        fn from(simplified_artist: &SimplifiedArtist) -> Self {
            Self {
                name: simplified_artist.name.clone(),
            }
        }
    }
}
