use std::{collections::HashSet, sync::RwLock};

use rocket::{
    http::Status,
    outcome::IntoOutcome,
    request::{self, FromRequest, Outcome},
    Request, State,
};
use rspotify::{clients::BaseClient, AuthCodeSpotify, Credentials, OAuth, Token};
use tokio::sync::Mutex;

pub struct AuthenticationState {
    oauth: OAuth,
    token: Option<Token>,
    creds: Credentials,
}

impl AuthenticationState {
    pub async fn new(client: AuthCodeSpotify) -> AuthenticationState {
        Self {
            oauth: client.oauth,
            token: client.token.lock().await.unwrap().clone(),
            creds: client.creds,
        }
    }
}

pub struct SpotifyClient {
    pub spotify: AuthCodeSpotify,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for SpotifyClient {
    type Error = anyhow::Error;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let auth_state = request
            .guard::<&State<Mutex<Option<AuthenticationState>>>>()
            .await;
        if !auth_state.is_success() {
            return Outcome::Failure((
                Status::InternalServerError,
                anyhow::Error::msg("failed to find static spotify authentication state"),
            ));
        }

        let mut maybe_spotify_auth = auth_state.unwrap().lock().await;
        if maybe_spotify_auth.is_none() {
            return Outcome::Failure((
                Status::Unauthorized,
                anyhow::Error::msg("the admin for this session does not have valid credentials"),
            ));
        }

        let mut spotify_auth = maybe_spotify_auth.as_mut().unwrap();

        let mut client =
            AuthCodeSpotify::new(spotify_auth.creds.clone(), spotify_auth.oauth.clone());
        *client.token.lock().await.unwrap() = spotify_auth.token.clone();

        client.auto_reauth().await.unwrap();

        spotify_auth.token = client.token.lock().await.unwrap().clone();

        Outcome::Success(SpotifyClient { spotify: client })
    }
}

pub fn scopes() -> HashSet<String> {
    let scopes = [
        "user-modify-playback-state",
        "user-read-playback-state",
        "user-read-currently-playing",
    ];
    return HashSet::from(scopes.map(|s| s.to_owned()));
}
