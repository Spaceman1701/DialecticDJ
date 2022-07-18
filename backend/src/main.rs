use persistence::DataStore;

use rspotify::{clients::OAuthClient, AuthCodeSpotify, Credentials, OAuth};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};

mod client;
mod persistence;
mod player;
mod routes;

#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    let client = initalize_spotify().await.unwrap();
    let spotify = Arc::new(client);
    let spotify_config = Arc::new(DjState {
        client: spotify.clone(),
        data_store: DataStore::new(),
    });

    let (tx, rx) = mpsc::channel::<NonEmptyQueueCommand>(2);

    start_player_thread(&spotify_config, rx);

    let player_cmd_tx = player::start_player_thread(spotify);

    rocket::build()
        .mount(
            "/",
            routes![
                routes::search,
                routes::play_track,
                routes::add_track_to_queue,
                routes::get_queued_tracks
            ],
        )
        .manage(spotify_config)
        .manage(player_cmd_tx)
}

fn start_player_thread(state: &Arc<DjState>, command_reciever: Receiver<NonEmptyQueueCommand>) {
    let cloned_state = state.clone();
    let mut rx = command_reciever;

    tokio::task::spawn(async move {
        loop {
            rx.recv().await;
            let first = cloned_state.data_store.pop_first_track().await.unwrap();
            println!("playing {}", first.name);

            cloned_state
                .client
                .add_item_to_queue(&first.id, None)
                .await
                .unwrap();

            tokio::time::sleep(first.duration - Duration::from_secs(10)).await;
        }
    });
}

pub struct NonEmptyQueueCommand;

pub struct DjState {
    client: Arc<AuthCodeSpotify>, //BaseClient requires "Clone" which means it can't be used as a dyn trait object :/
    // Seriously consider forking the library to solve this problem
    data_store: DataStore,
}

async fn initalize_spotify() -> Option<AuthCodeSpotify> {
    let creds = Credentials::from_env();
    if creds.is_none() {
        println!("No credentials found in the enviornment, crashing!");
        return None;
    }
    let oauth_info = OAuth::from_env(scopes()).unwrap();
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

fn scopes() -> HashSet<String> {
    let scopes = [
        "user-modify-playback-state",
        "user-read-playback-state",
        "user-read-currently-playing",
    ];
    return HashSet::from(scopes.map(|s| s.to_owned()));
}
