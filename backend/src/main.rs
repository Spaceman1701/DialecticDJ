use rspotify::{ClientCredsSpotify, Credentials, Token, clients::BaseClient};

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello, get_auth])
}

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}

#[get("/auth")]
async fn get_auth() -> Option<String> {
    let creds = Credentials::from_env();
    if creds.is_none() {
        return None;
    }
    let mut spotify_client = ClientCredsSpotify::new(creds.unwrap());
    let token_response = spotify_client.request_token();
    let token_result = token_response.await;
    if token_result.is_err() {
        println!("AUTH ERROR: {}", token_result.err().unwrap());
        return None;
    }

    let search = spotify_client.search("Gordon Lightfoot", &rspotify::model::SearchType::Artist, None, None, Some(5), None).await;
    if search.is_err() {
        println!("SEARCH ERROR: {}", search.err().unwrap());
        return None;
    }
    let real_search = search.unwrap();

    return Some(format!("{:#?}", real_search));
}
