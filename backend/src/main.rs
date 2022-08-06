#![feature(associated_type_bounds)]

use authentication::ManagedAuthState;

use rocket::fairing::AdHoc;

use rocket::http::Header;
use rspotify::Credentials;

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
    let creds = Credentials::from_env();
    if creds.is_none() {
        panic!("can't start server without available spotify app credentials");
    }
    let config = rocket::Config {
        address: std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        port: 8090,
        ..Default::default()
    };

    let auth: ManagedAuthState = Arc::default();
    let player_cmd = player::start_player_thread(auth.clone());

    let data_store_result = persistence::pgsql::PostgressDatabase::connect().await;
    if let Err(e) = data_store_result {
        panic!("failed to connect to database: {}", e);
    }
    let data_store = data_store_result.unwrap();
    let create_tables_res = data_store.create_tables().await;
    if let Err(e) = create_tables_res {
        panic!("failed to create tables: {}", e);
    }

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
                routes::create_session,
                routes::authenticate_session
            ],
        )
        .manage(player_cmd)
        .manage(auth)
        .manage(data_store)
        .configure(config)
        .attach(AdHoc::on_response("CORS Headers", |_, response| {
            Box::pin(async move {
                response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
                response.set_header(Header::new(
                    "Access-Control-Allow-Methods",
                    "POST, GET, PATCH, OPTIONS",
                ));
                response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
                response
                    .set_header(Header::new("Access-Control-Allow-Credentials", "true"));
            })
        }))
}
