// (Lines like the one below ignore selected Clippy rules
//  - it's useful when you want to check your code with `cargo make verify`
// but some rules are too "annoying" or are not applicable for your case.)
#![allow(clippy::wildcard_imports)]

use ddj_core::types::{PlayerState, Track};
use seed::{prelude::*, *};

// ------ ------
//     Init
// ------ ------

// `init` describes what should happen when your app started.
fn init(_: Url, orders: &mut impl Orders<Msg>) -> Model {
    orders.perform_cmd(async { Msg::NewStateAvailable(request_new_state().await) });

    Model {
        loaded: LoadingState::Loading,
        currently_playing: None,
        queue: Vec::new(),
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
struct Model {
    loaded: LoadingState,
    currently_playing: Option<Track>,
    queue: Vec<Track>,
}

// ------ ------
//    Update
// ------ ------

// (Remove the line below once any of your `Msg` variants doesn't implement `Copy`.)
// `Msg` describes the different events you can modify state with.
enum Msg {
    NewStateAvailable(fetch::Result<PlayerState>),
}

// `update` describes how to handle each `Msg`.
fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    // if model.loaded == LoadingState::Loading {
    //     orders.perform_cmd(async { Msg::NewStateAvailable(request_new_state().await) });
    //     return;
    // }
    println!("requesting state");
    model.loaded = LoadingState::Error("bad".to_owned());
    match msg {
        Msg::NewStateAvailable(fetch_result) => match fetch_result {
            Ok(player_state) => {
                model.loaded = LoadingState::Done;
                model.currently_playing = player_state.current_track;
                model.queue = player_state.queue;
            }
            Err(_) => model.loaded = LoadingState::Error("player state fetch failed".to_owned()),
        },
    }
}

async fn request_new_state() -> fetch::Result<PlayerState> {
    let request = Request::new("http://127.0.0.1:8000/current_state").method(Method::Get);

    let response = fetch(request).await?;
    let payload = response.json().await?;

    Ok(payload)
}

// ------ ------
//     View
// ------ ------

// `view` describes what to display.
fn view(model: &Model) -> Node<Msg> {
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
    div![
        C!["track"],
        img!(attrs! {At::Src => album_art}),
        div![C!["track-info"], div![&track.name], div![duration_str]],
    ]
}

fn view_plus_button() -> Node<Msg> {
    div![C!["plus-button"], div!["+"]]
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
