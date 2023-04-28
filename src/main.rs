#[macro_use] extern crate rocket;
#[macro_use] extern crate rocket_sync_db_pools;

use std::thread::current;

use busses::busses_data;
use rocket::{Rocket, Build};
use rocket::fairing::AdHoc;
use rocket::response::{Debug, status::Created};
use rocket::serde::{Serialize, Deserialize, json::Json};

use rocket_sync_db_pools::diesel;

use self::diesel::prelude::*;

mod routes;
mod places;
mod busses;
use places::place_data;
use routes::route_data;
mod bus;
use bus::bus_data;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(bus_data())
        .attach(route_data())
        .attach(place_data())
        .attach(busses_data())
}