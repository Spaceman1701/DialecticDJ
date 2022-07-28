use ddj_core::types::Track;
use sqlx::{postgres::PgRow, ColumnIndex, FromRow, Row};
use std::time::Duration;

use rocket::serde::{Deserialize, Serialize};
use rspotify::model::{FullTrack, Id, SimplifiedAlbum, TrackId};

#[repr(transparent)]
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SpotifyItemId(pub String);

impl SpotifyItemId {
    pub fn into_id<T: Id>(&self) -> T {
        let res = T::from_id(&self.0);
        if let Err(_) = res {
            panic!("invalid characters found in a spotify item id");
        }
        return res.unwrap();
    }
}

impl<T: Id> From<T> for SpotifyItemId {
    fn from(id: T) -> Self {
        let id_str = id.id().to_owned();
        return Self(id_str);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackInfo {
    pub id: SpotifyItemId,
    pub name: String,
    pub duration: Duration,
    pub album: Album,
}

impl From<FullTrack> for TrackInfo {
    fn from(track: FullTrack) -> Self {
        return TrackInfo {
            id: track.id.unwrap().into(),
            name: track.name,
            duration: track.duration,
            album: track.album.into(),
        };
    }
}

// impl<'r> FromRow<'r, PgRow> for TrackInfo {
//     fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
//         let id = row.try_get("id")?;
//         let name = row.try_get("name")?;
//         let duration = row.try_get("duration")?;
//     }
// }

impl Into<Track> for &TrackInfo {
    fn into(self) -> Track {
        Track {
            name: self.name.clone(),
            id: self.id.0.clone(),
            artists: Vec::new(),
            duration: self.duration,
            album_art_link: self.album.first_image_url.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Album {
    id: SpotifyItemId,
    name: String,
    first_image_url: Option<String>,
}

impl From<SimplifiedAlbum> for Album {
    fn from(input: SimplifiedAlbum) -> Self {
        Self {
            id: input.id.unwrap().into(),
            name: input.name,
            first_image_url: input.images.first().map(|image| image.url.clone()),
        }
    }
}

impl<'r> FromRow<'r, PgRow> for Album {
    fn from_row(row: &'r PgRow) -> Result<Self, sqlx::Error> {
        let id = row.try_get("id")?;
        let name = row.try_get("name")?;
        let first_image_url = row.try_get("cover_image_url")?;

        return Ok(Album {
            id: SpotifyItemId(id),
            name: name,
            first_image_url: first_image_url,
        });
    }
}
