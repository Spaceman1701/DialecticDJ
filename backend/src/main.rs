use std::ops::Deref;

use rocket::State;
use rocket::serde::json::Json;
use rspotify::{ClientCredsSpotify, Credentials, Token, clients::{BaseClient, OAuthClient}, OAuth, AuthCodeSpotify};
use core::{self, DDJ::SearchResult};

mod client;

#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    let client = initalize_spotify().await.unwrap();
    let spotify_config = SpotifyConfig {
        client,
    };

    rocket::build().mount("/", routes![search]).manage(spotify_config)
}

struct SpotifyConfig {
    client: ClientCredsSpotify, //BaseClient requires "Clone" which means it can't be used as a dyn trait object :/
    // Seriously consider forking the library to solve this problem
}



async fn initalize_spotify() -> Option<ClientCredsSpotify> {

    let creds = Credentials::from_env();
    if creds.is_none() {
        println!("No credentials found in the enviornment, crashing!");
        return None;
    }
    let mut spotify_client = ClientCredsSpotify::new(creds.unwrap());
    let token_response = spotify_client.request_token();
    let token_result = token_response.await;
    if let Err(err) = token_result {
        println!("Spotify auth failed: {}", err);
        None
    } else {
        Some(spotify_client)
    }
}


#[post("/search", data = "<query>")]
async fn search(state: &State<SpotifyConfig>, query: String) -> Option<Json<SearchResult>> {
    let search = state.client.search(&query, &rspotify::model::SearchType::Track, None, None, Some(5), None).await;
    if let Err(err) = search {
        println!("SEARCH ERROR: {}", err);
        return None;
    }
    let real_search = search.unwrap();

    let wrapped_result = SearchResult(real_search);

    return Some(Json(wrapped_result));
}
