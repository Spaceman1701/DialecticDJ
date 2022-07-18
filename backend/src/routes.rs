use core::DialecticDj::SearchResult;
use std::sync::Arc;

use rocket::{response::status::BadRequest, serde::json::Json, State};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{Id, TrackId},
};
use tokio::sync::mpsc::{self, Sender};

use crate::{
    persistence::TrackInfo,
    player::{PlayerCommand, PlayerCommandQueue},
    DjState, NonEmptyQueueCommand, PlayerCommandBuffer,
};

#[post("/search", data = "<query>")]
pub async fn search(state: &State<Arc<DjState>>, query: String) -> Option<Json<SearchResult>> {
    let search = state
        .client
        .search(
            &query,
            &rspotify::model::SearchType::Track,
            None,
            None,
            Some(5),
            None,
        )
        .await;
    if let Err(err) = search {
        println!("SEARCH ERROR: {}", err);
        return None;
    }
    let real_search = search.unwrap();

    return if let rspotify::model::SearchResult::Tracks(tracks) = real_search {
        let items = tracks.items;
        let final_result: SearchResult = SearchResult::from(items);
        Some(Json(final_result))
    } else {
        panic!("track search somehow returned non-track results")
    };
}

#[post("/play/<track_id>")]
pub async fn play_track(state: &State<Arc<DjState>>, track_id: String) {
    let id = TrackId::from_id(&track_id).unwrap();
    state.client.add_item_to_queue(&id, None).await.unwrap();
    state.client.next_track(None).await.unwrap();
}

#[post("/queue/<track_id>")]
pub async fn add_track_to_queue(
    player_cmds: &State<PlayerCommandQueue>,
    track_id: String,
) -> Result<(), BadRequest<()>> {
    let id = TrackId::from_id(&track_id);
    match id {
        Err(_) => Err(BadRequest(None)),
        Ok(unwrapped_id) => {
            player_cmds
                .send(PlayerCommand::AddTrack(unwrapped_id))
                .await
                .unwrap();
            Ok(())
        }
    }
}

#[get("/queue")]
pub async fn get_queued_tracks(state: &State<Arc<DjState>>) -> Json<Vec<TrackInfo>> {
    let data = state.data_store.get_all_tracks().await;
    return Json(data);
}