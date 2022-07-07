
#[macro_use] extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello])
}

#[get("/<name>/<age>")]
fn hello(name: &str, age: u8) -> String {
    format!("Hello, {} year old named {}!", age, name)
}
