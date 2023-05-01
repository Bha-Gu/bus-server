#[macro_use]
extern crate rocket;
#[macro_use]
extern crate rocket_sync_db_pools;

use busses::busses_data;

use rocket::fairing::AdHoc;

use rocket_sync_db_pools::diesel;

mod busses;
mod places;
mod routes;
use places::place_data;
use routes::route_data;
mod bus;
use bus::bus_data;
use rocket::http;
use rocket_cors::{AllowedHeaders, AllowedOrigins, Cors, Method};

fn cors() -> Cors {
    rocket_cors::CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![http::Method::Get, http::Method::Post, http::Method::Options]
            .into_iter()
            .map(Method)
            .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        fairing_route_base: "/".to_owned(),
        max_age: Some(42),
        ..Default::default()
    }
    .to_cors()
    .unwrap()
}

fn stage() -> AdHoc {
    AdHoc::on_ignite("Rusqlite Stage", |rocket| async {
        rocket
            .manage(cors())
            .mount("/", rocket_cors::catch_all_options_routes())
    })
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(stage())
        .attach(bus_data())
        .attach(route_data())
        .attach(place_data())
        .attach(busses_data())
}
