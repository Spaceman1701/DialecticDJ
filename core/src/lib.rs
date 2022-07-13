pub mod DDJ {
    use rspotify;
    use serde::{Deserialize, Serialize};

    #[repr(transparent)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct SearchResult(pub rspotify::model::SearchResult);

    #[repr(transparent)]
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Track(pub rspotify::model::SimplifiedTrack);
}
