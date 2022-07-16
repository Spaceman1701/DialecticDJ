pub mod DialecticDj {
    use std::{collections::HashMap, time::Duration};

    use rspotify::{
        self,
        model::{Restriction, SimplifiedAlbum, FullTrack, Id},
    };
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[repr(transparent)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SearchResult {
        tracks: Vec<DJTrack>,
    }

    #[repr(transparent)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Track(pub rspotify::model::FullTrack);

    pub type PropMap = HashMap<String, String>;

    #[derive(Deserialize, Debug, Serialize)]
    pub struct ArtistId(pub String);

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TrackId(pub String);

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DJTrack {
        pub album: SimplifiedAlbum,
        pub artists: Vec<ArtistId>,
        pub available_markets: Vec<String>,
        pub duration: Duration,
        pub explicit: bool,
        pub external_ids: PropMap,
        pub external_urls: PropMap,
        pub href: Option<String>,
        pub id: TrackId,
        pub restrictions: Option<Restriction>,
        pub name: String,
        pub popularity: u32,
        pub preview_url: Option<String>,
        pub track_number: u32,
    }

    impl DJTrack {
        pub fn from_rspotify(source: &FullTrack) -> Option<DJTrack> {
            let mut artist_ids = Vec::new();
            for artist in source.artists.iter() {
                if let Some(id) = &artist.id {
                    let raw_id = id.id().to_owned();
                    artist_ids.push(ArtistId(raw_id));
                }
            }

            if source.id.is_none() {
                return None; //Can't use tracks which don't have an ID in the DJ model
            }


            return Some(DJTrack {
                album: source.album.clone(),
                artists: artist_ids,
                available_markets: Vec::new(),
                duration: source.duration,
                explicit: source.explicit,
                external_ids: source.external_ids.clone(),
                external_urls: source.external_urls.clone(),
                href: source.href.clone(),
                id: TrackId(source.id.as_ref().unwrap().id().to_owned()),
                restrictions: None,
                name: source.name.clone(),
                popularity: source.popularity,
                preview_url: source.preview_url.clone(),
                track_number: source.track_number,
            });
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct DJPage<T> {
        pub items: Vec<T>,
        pub limit: u32,
        pub next: Option<String>,
        pub prev: Option<String>,
        pub offset: u32,
        pub total: u32,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Session {
        id: Uuid,
        tracks: Vec<Track>,
    }
}
