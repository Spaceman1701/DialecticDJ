// (Lines like the one below ignore selected Clippy rules
//  - it's useful when you want to check your code with `cargo make verify`
// but some rules are too "annoying" or are not applicable for your case.)
#![allow(clippy::wildcard_imports)]

use std::str::FromStr;

use ddj_core::types::{
    AuthenticateClientMessage, CreateSessionResponse, PlayerState, Session, Track,
};
use seed::{prelude::*, *};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SEARCH: &str = "search";
const DEVICE_SELECTION: &str = "device_selection";
const LOGIN: &str = "login";

// ------ ------
//     Init
// ------ ------

// `init` describes what should happen when your app started.
fn init(url: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.stream(streams::interval(10000, || Msg::UpdateState));

    let page = match url.hash_path().get(0) {
        Some(path) => {
            if path == LOGIN {
                log!("login page init");
                match url.search().get("code") {
                    Some(values) => {
                        let first = values.first();
                        log!(first);
                        match first {
                            Some(code) => {
                                let cloned = code.clone();
                                orders.perform_cmd(async move {
                                    

                                    send_code(&cloned).await;
                                });
                                Page::Landing
                            }
                            None => Page::Landing,
                        }
                    }
                    None => {
                        orders.perform_cmd(async {
                            Msg::AuthUrlAvailable(request_login_url().await)
                        });

                        Page::Login(None)
                    }
                }
            } else {
                Page::Landing
            }
        }
        None => Page::Landing,
    };

    if page == Page::Landing {
        update_state(orders);
        Url::from_str("http://192.168.0.22:8080/")
            .unwrap()
            .go_and_replace();
    }

    Model {
        page: page,
        loaded: LoadingState::Loading,
        currently_playing: None,
        queue: Vec::new(),
        search_model: SearchModel {
            results: Vec::new(),
            in_progress: false,
            error: None,
        },
        session: None,
    }
}

// ------ ------
//     Model
// ------ ------

// `Model` describes our app state.
#[derive(PartialEq, Eq)]
enum LoadingState {
    Done,
    Error(String),
    Loading,
}

#[derive(PartialEq, Eq)]
enum Page {
    Landing,
    Search(bool),
    DeviceSelection,
    Login(Option<String>),
}

#[derive(PartialEq, Eq)]
enum Mode {
    Search(bool),
    Normal,
}
struct Model {
    page: Page,
    loaded: LoadingState,
    currently_playing: Option<Track>,
    queue: Vec<Track>,
    search_model: SearchModel,
    session: Option<Session>,
}

struct SearchModel {
    results: Vec<Track>,
    in_progress: bool,
    error: Option<String>,
}

// ------ ------
//    Update
// ------ ------

// (Remove the line below once any of your `Msg` variants doesn't implement `Copy`.)
// `Msg` describes the different events you can modify state with.
enum Msg {
    NewStateAvailable(fetch::Result<PlayerState>),
    EnterSearchMode,
    SearchInputChanged(String),
    SearchResultAvailable(fetch::Result<Vec<Track>>),
    TrackClicked(Track),
    UpdateState,
    AuthUrlAvailable(fetch::Result<CreateSessionResponse>),
}

// `update` describes how to handle each `Msg`.
fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::NewStateAvailable(fetch_result) => match fetch_result {
            Ok(player_state) => {
                model.loaded = LoadingState::Done;
                model.currently_playing = player_state.current_track;
                model.queue = player_state.queue;
            }
            Err(_) => {
                model.loaded = LoadingState::Error("player state fetch failed".to_owned())
            }
        },
        Msg::EnterSearchMode => {
            model.page = Page::Search(false);
        }
        Msg::SearchInputChanged(new_input) => {
            if model.page == Page::Search(false) {
                orders.perform_cmd(async move {
                    Msg::SearchResultAvailable(search(&new_input).await)
                });
                model.search_model.in_progress = true;
            }
        }
        Msg::SearchResultAvailable(fetch_result) => match fetch_result {
            Ok(search_result) => {
                model.search_model.in_progress = false;
                model.search_model.results = search_result;
            }
            Err(_) => {
                model.search_model.error = Some("search query failed".to_owned());
            }
        },
        Msg::TrackClicked(track) => match model.page {
            Page::Search(_) => {
                orders.perform_cmd(async move {
                    add_track_to_queue(&track).await;
                });
                model.page = Page::Landing;
                update_state(orders);
            }
            _ => {}
        },
        Msg::UpdateState => {
            orders.skip();
            match model.page {
                Page::Landing => {
                    update_state(orders);
                }
                _ => (),
            }
        }
        Msg::AuthUrlAvailable(url) => match url {
            Ok(session) => {
                log!("got auth url");
                model.page = Page::Login(Some(session.auth_link));
                model.session = Some(session.session);
            }
            Err(err) => {
                log!("failed to recieve redirect URL: {}",);
            }
        },
    }
}

fn update_state(orders: &mut impl Orders<Msg>) {
    orders.perform_cmd(async { Msg::NewStateAvailable(request_new_state().await) });
}

const BASE_URL: &str = "http://192.168.0.22:8090";

async fn request_new_state() -> fetch::Result<PlayerState> {
    let request = Request::new(format!("{}/current_state", BASE_URL)).method(Method::Get);

    let response = fetch(request).await?;
    let payload = response.json().await?;

    Ok(payload)
}

async fn search(query: &str) -> fetch::Result<Vec<Track>> {
    let request = Request::new(format!("{}/search", BASE_URL))
        .method(Method::Post)
        .json(query)?;

    let response = fetch(request).await?;
    let payload = response.json().await?;

    Ok(payload)
}

async fn add_track_to_queue(track: &Track) -> fetch::Result<()> {
    let request =
        Request::new(format!("{}/queue/{}", BASE_URL, &track.id)).method(Method::Post);
    fetch(request).await?;

    Ok(())
}

async fn request_login_url() -> fetch::Result<CreateSessionResponse> {
    let request = Request::new(format!("{}/new_session/{}", BASE_URL, "test-session"))
        .method(Method::Post);
    let response = fetch(request).await?;
    let payload = response.json().await?;
    Ok(payload)
}

async fn send_code(code: &str, session_id: Uuid) {
    let message = AuthenticateClientMessage {
        session_id: session_id,
        auth_code: code.to_owned(),
    };
    let request = Request::new(format!(
        "{}/authenticate_session/{}",
        BASE_URL,
        session_id.to_string()
    ))
    .method(Method::Post)
    .json(&message)
    .unwrap();
    let response = fetch(request).await.unwrap();
}

// ------ ------
//     View
// ------ ------

// `view` describes what to display.
fn view(model: &Model) -> Node<Msg> {
    match &model.page {
        Page::Landing => view_normal_mode(model),
        Page::Search(_) => view_search_mode(model),
        Page::DeviceSelection => todo!(),
        Page::Login(url) => div![
            "redirecting...",
            match url {
                Some(real_url) => a![attrs!(At::Href => real_url), "click here!"],
                None => div![],
            }
        ],
    }
}

fn view_search_mode(model: &Model) -> Node<Msg> {
    div![
        C!["content"],
        div![C!["app-title"], "Add Something"],
        input![
            C!["search-input"],
            "search",
            input_ev(Ev::Input, move |input| Msg::SearchInputChanged(input))
        ],
        div![
            C!["search-result-container"],
            view_search_results(&model.search_model)
        ]
    ]
}

fn view_search_results(model: &SearchModel) -> Node<Msg> {
    if let Some(err) = &model.error {
        div![format!("ERROR: {}", err)]
    } else {
        view_queue(&model.results)
    }
}

fn view_normal_mode(model: &Model) -> Node<Msg> {
    div![
        C!["content"],
        div![C!["app-title"], "Dialectic DJ"],
        div![match &model.loaded {
            LoadingState::Done => {
                div![
                    div![view_currently_playing(&model.currently_playing)],
                    div![format!("{} Songs in Queue:", model.queue.len())],
                    view_queue(&model.queue)
                ]
            }
            LoadingState::Error(msg) => {
                div![format!("loading failed: {}", msg)]
            }
            LoadingState::Loading => {
                div!["loading....."]
            }
        }],
        view_plus_button()
    ]
}

fn view_queue(queue: &Vec<Track>) -> Node<Msg> {
    div![queue.iter().map(|track| view_track(track))]
}

fn view_currently_playing(track: &Option<Track>) -> Node<Msg> {
    if let Some(inner_track) = track {
        div![div!["Currently playing:"], view_track(&inner_track)]
    } else {
        div!["Nothing Playing"]
    }
}

fn view_track(track: &Track) -> Node<Msg> {
    let album_art = track.album_art_link.as_ref().unwrap();
    let duration_mins = track.duration.as_secs() / 60;
    let duration_secs = track.duration.as_secs() % 60;
    let duration_str = format!("{}:{}", duration_mins, duration_secs);
    let cloned_track = track.clone();
    div![
        C!["track"],
        img!(attrs! {At::Src => album_art}),
        div![C!["track-info"], div![&track.name], div![duration_str]],
        ev(Ev::Click, move |_| { Msg::TrackClicked(cloned_track) })
    ]
}

fn view_plus_button() -> Node<Msg> {
    div![
        C!["plus-button"],
        div!["+", ev(Ev::Click, |_| Msg::EnterSearchMode)]
    ]
}

// ------ ------
//     Start
// ------ ------

// (This function is invoked by `init` function in `index.html`.)
#[wasm_bindgen(start)]
pub fn start() {
    // Mount the `app` to the element with the `id` "app".
    App::start("app", init, update, view);
}
