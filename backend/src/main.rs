use persistence::DataStore;

use rocket::fairing::AdHoc;
use rocket::http::Header;
use rspotify::{clients::OAuthClient, AuthCodeSpotify, Credentials, OAuth};
use std::collections::HashSet;
use std::sync::Arc;

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

    let player_cmd = player::start_player_thread(spotify);

    rocket::build()
        .mount(
            "/",
            routes![
                routes::search,
                routes::play_track,
                routes::add_track_to_queue,
                routes::get_queued_tracks,
                routes::get_current_state
            ],
        )
        .manage(spotify_config)
        .manage(player_cmd)
        .attach(AdHoc::on_response("CORS Headers", |req, response| {
            Box::pin(async move {
                response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
                response.set_header(Header::new(
                    "Access-Control-Allow-Methods",
                    "POST, GET, PATCH, OPTIONS",
                ));
                response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
                response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
            })
        }))
}

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
