use std::{str::FromStr, time::Duration};

use ddj_core::types::{CreateSessionResponse, PlayerState, Session, Track};
use rocket::{http::Status, response::status::BadRequest, serde::json::Json, State};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{Id, TrackId},
    AuthCodeSpotify, Credentials, OAuth,
};

use crate::{
    authentication::{self, AuthenticationState, ManagedAuthState, SpotifyClient},
    model::TrackInfo,
    persistence::{
        model::{SpotifyAlbum, SpotifyTrack},
        Store,
    },
    player::PlayerCommader,
};

#[post("/search", data = "<query>")]
pub async fn search(client: SpotifyClient, query: String) -> Option<Json<Vec<Track>>> {
    let search = client
        .spotify
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
        let final_result: Vec<Track> = items.iter().map(|ft| ft.into()).collect();
        Some(Json(final_result))
    } else {
        panic!("track search somehow returned non-track results")
    };
}

#[post("/next_track")]
pub async fn play_track(state: &State<PlayerCommader>) {
    state.start().await.unwrap();
}

#[post("/queue/<track_id>")]
pub async fn add_track_to_queue(
    player_cmd: &State<PlayerCommader>,
    store: &State<Store>,
    track_id: String,
) -> Result<(), BadRequest<()>> {
    let id = TrackId::from_id(&track_id);
    match id {
        Err(_) => Err(BadRequest(None)),
        Ok(unwrapped_id) => {
            player_cmd.add_track_to_queue(unwrapped_id).await.unwrap();
            store
                .add_track_to_queue(SpotifyTrack {
                    id: track_id,
                    name: "n/a".to_owned(),
                    duration: Duration::from_secs(0),
                    album: SpotifyAlbum {
                        name: "n/a".to_owned(),
                        id: "n/a".to_owned(),
                        cover_image_url: "n/a".to_owned(),
                    },
                })
                .await
                .unwrap();
            Ok(())
        }
    }
}

#[get("/queue")]
pub async fn get_queued_tracks(state: &State<PlayerCommader>) -> Json<Vec<TrackInfo>> {
    let data = state.get_queued_tracks().await.unwrap();
    return Json(data);
}

#[get("/current_state")]
pub async fn get_current_state(
    player: &State<PlayerCommader>,
) -> Json<ddj_core::types::PlayerState> {
    let current_track = player.get_currently_playing_track().await.unwrap();
    let unwrapped: Option<Track> = current_track.map(|track: TrackInfo| (&track).into());

    let queue = player.get_queued_tracks().await.unwrap();
    let transformed_queue: Vec<Track> = queue.iter().map(|info| info.into()).collect();
    println!("found {} in queue", transformed_queue.len());
    return Json(PlayerState {
        current_track: unwrapped,
        queue: transformed_queue,
    });
}

#[options("/<_..>")]
pub async fn handle_options<'a>() -> () {
    ()
}

#[post("/start_auth_flow")]
pub async fn start_auth_flow() -> (Status, Option<String>) {
    let creds = Credentials::from_env();
    if creds.is_none() {
        println!("No credentials found in the enviornment, crashing!");
        return (Status::InternalServerError, None);
    }
    let mut oauth_info = OAuth::from_env(authentication::scopes()).unwrap();
    oauth_info.redirect_uri = "http://192.168.0.22:8080#login".to_owned();

    let client = AuthCodeSpotify::new(creds.unwrap(), oauth_info);
    let authorize_url = client.get_authorize_url(true).unwrap();

    (Status::Ok, Some(authorize_url))
}

#[post("/finish_auth_flow/<code>")]
pub async fn finish_auth_flow(code: String, auth: &State<ManagedAuthState>) {
    let creds = Credentials::from_env();
    if creds.is_none() {
        panic!("No credentials found in the enviornment, crashing!");
    }
    let mut oauth_info = OAuth::from_env(authentication::scopes()).unwrap();
    oauth_info.redirect_uri = "http://192.168.0.22:8080#login".to_owned();
    let mut client = AuthCodeSpotify::new(creds.unwrap(), oauth_info);

    client.request_token(&code).await.unwrap();

    let auth_state = AuthenticationState::new(client).await;
    *auth.lock().await = Some(auth_state);
    println!("successfully authenticated client")
}

#[post("/new_session/<name>")]
pub async fn create_session(
    name: &str,
    store: &State<Store>,
) -> Result<Json<CreateSessionResponse>, Status> {
    let creds = Credentials::from_env().unwrap();
    let mut oauth_info = OAuth::from_env(authentication::scopes()).unwrap();
    oauth_info.redirect_uri = "http://192.168.0.22:8080#login".to_owned();
    let client = AuthCodeSpotify::new(creds, oauth_info);
    let authorize_url = client.get_authorize_url(true).unwrap();

    let res = store.create_session(name).await;
    match res {
        Ok(session) => Ok(Json(CreateSessionResponse {
            session: Session {
                id: session.id,
                name: name.to_owned(),
            },
            auth_link: authorize_url,
        })),
        Err(e) => {
            println!("failed to create session: {}", e);
            Err(Status::InternalServerError)
        }
    }
}

#[post("/authenticate_session/<id>", data = "<code>")]
pub async fn authenticate_session(id: &str, code: &str, store: &State<Store>) {
    let creds = Credentials::from_env().unwrap();
    let mut oauth_info = OAuth::from_env(authentication::scopes()).unwrap();
    oauth_info.redirect_uri = "http://192.168.0.22:8080#login".to_owned();
    let mut client = AuthCodeSpotify::new(creds, oauth_info);
    client.request_token(&code).await.unwrap();
    let session_id = uuid::Uuid::from_str(id).unwrap();

    let mut session = store.get_session(session_id).await.unwrap().unwrap();

    session.token = client.token.lock().await.unwrap().clone();

    store.update_session(&session).await.unwrap();
}
