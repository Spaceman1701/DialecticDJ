use anyhow::Result;
use core::{self, DialecticDj::SearchResult};
use persistence::{DataStore, TrackInfo};
use rocket::tokio::sync::RwLock;
use rocket::State;
use rocket::{response::status::BadRequest, serde::json::Json};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{Id, TrackId},
    AuthCodeSpotify, ClientCredsSpotify, Credentials, OAuth,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::mpsc::{self, Sender};

mod client;
mod persistence;

#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    let client = initalize_spotify().await.unwrap();
    let spotify_config = SpotifyConfig {
        client,
        data_store: DataStore::new(),
    };

    let (tx, mut rx) = mpsc::channel::<NonEmptyQueueCommand>(1);

    // let background_state = spotify_config.clone();
    // tokio::task::spawn(async move {
    //     loop {
    //         rx.recv().await;
    //         let first = background_state.data_store.pop_first_track().await.unwrap();
    //         println!("playing {}", first.name);
    //         tokio::time::sleep(first.duration);
    //     }
    // });

    rocket::build()
        .mount(
            "/",
            routes![search, play_track, add_track_to_queue, get_queued_tracks],
        )
        .manage(spotify_config)
        .manage(PlayerCommandBuffer { tx: tx })
}

struct PlayerCommandBuffer {
    tx: Sender<NonEmptyQueueCommand>,
}

struct NonEmptyQueueCommand;

struct SpotifyConfig {
    client: AuthCodeSpotify, //BaseClient requires "Clone" which means it can't be used as a dyn trait object :/
    // Seriously consider forking the library to solve this problem
    data_store: DataStore,
}

async fn initalize_spotify() -> Option<AuthCodeSpotify> {
    let creds = Credentials::from_env();
    if creds.is_none() {
        println!("No credentials found in the enviornment, crashing!");
        return None;
    }
    let scopes = HashSet::from(["user-modify-playback-state".to_owned()]);

    let oauth_info = OAuth::from_env(scopes).unwrap();
    let mut client = AuthCodeSpotify::new(creds.unwrap(), oauth_info);
    let authorize_url = client.get_authorize_url(true).unwrap();
    println!("authorize url: {}", authorize_url);
    println!("enter return code: ");

    let mut code = String::new();
    std::io::stdin().read_line(&mut code).unwrap();
    let response_code = client.parse_response_code(&code).unwrap();
    client.request_token(&response_code).await.unwrap();

    return Some(client);
}

#[post("/search", data = "<query>")]
async fn search(state: &State<SpotifyConfig>, query: String) -> Option<Json<SearchResult>> {
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
async fn play_track(state: &State<SpotifyConfig>, track_id: String) {
    let id = TrackId::from_id(&track_id).unwrap();
    state.client.add_item_to_queue(&id, None).await.unwrap();
    state.client.next_track(None).await.unwrap();
}

#[post("/queue/<track_id>")]
async fn add_track_to_queue(
    state: &State<SpotifyConfig>,
    track_id: String,
) -> Result<(), BadRequest<()>> {
    let id = TrackId::from_id(&track_id);
    match id {
        Err(_) => Err(BadRequest(None)),
        Ok(unwrapped_id) => {
            let track_result = state.client.track(&unwrapped_id).await;
            if let Err(err) = track_result {
                return Err(BadRequest(None));
            }
            let full_track = track_result.unwrap();
            state.data_store.add_track(full_track.into()).await;

            tokio::task::spawn(async {});

            Ok(())
        }
    }
}

#[get("/queue")]
async fn get_queued_tracks(state: &State<SpotifyConfig>) -> Json<Vec<TrackInfo>> {
    let data = state.data_store.get_all_tracks().await;
    return Json(data);
}
