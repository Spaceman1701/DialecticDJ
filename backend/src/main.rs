use authentication::ManagedAuthState;

use rocket::fairing::AdHoc;

use rocket::http::Header;

use std::net::Ipv4Addr;
use std::sync::Arc;

mod authentication;
mod model;
mod persistence;
mod player;
mod routes;

#[macro_use]
extern crate rocket;

#[launch]
async fn rocket() -> _ {
    let config = rocket::Config {
        address: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        port: 8090,
        ..Default::default()
    };

    let auth: ManagedAuthState = Arc::default();
    let player_cmd = player::start_player_thread(auth.clone());

    rocket::build()
        .mount(
            "/",
            routes![
                routes::search,
                routes::play_track,
                routes::add_track_to_queue,
                routes::get_queued_tracks,
                routes::get_current_state,
                routes::handle_options,
                routes::start_auth_flow,
                routes::finish_auth_flow,
            ],
        )
        .manage(player_cmd)
        .manage(auth)
        .configure(config)
        .attach(AdHoc::on_response("CORS Headers", |_, response| {
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
