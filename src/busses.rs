use rocket::fairing::AdHoc;
use rocket::response::{Debug, Responder};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{http, Build, Rocket};

use self::diesel::prelude::*;
use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions, Method};
use rocket_sync_db_pools::diesel;

#[database("diesel")]
struct Db(diesel::SqliteConnection);

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = busses)]
struct Busses {
    placeid: String,
    busid: String,
}

table! {
    busses (placeid) {
        placeid -> Text,
        busid -> Text,
    }
}

fn core_options() -> CorsOptions {
    rocket_cors::CorsOptions {
        allowed_origins: AllowedOrigins::all(),
        allowed_methods: vec![
            http::Method::Get,
            http::Method::Post,
            http::Method::Options,
            http::Method::Delete,
        ]
        .into_iter()
        .map(Method)
        .collect(),
        allowed_headers: AllowedHeaders::all(),
        allow_credentials: true,
        fairing_route_base: "/".to_owned(),
        max_age: Some(42),
        ..Default::default()
    }
}

#[get("/")]
async fn list<'r, 'o: 'r>(db: Db) -> Result<impl Responder<'r, 'o>> {
    let ids: Vec<String> = db
        .run(move |conn| busses::table.select(busses::placeid).load(conn))
        .await?;

    let out = Json(ids);
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[get("/<id>")]
async fn get_one_bus<'r, 'o: 'r>(db: Db, id: String) -> Result<impl Responder<'r, 'o>> {
    let out: Json<Busses> = db
        .run(move |conn| busses::table.filter(busses::placeid.eq(id)).first(conn))
        .await
        .map(Json)?;

    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[delete("/<id>")]
async fn delete_one_bus<'r, 'o: 'r>(db: Db, id: String) -> Result<impl Responder<'r, 'o>> {
    let out: usize = db
        .run(move |conn| {
            diesel::delete(busses::table)
                .filter(busses::busid.eq(id))
                .execute(conn)
        })
        .await?;

    let out = (out == 1).then_some(());
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    Db::get_one(&rocket)
        .await
        .expect("database connection")
        .run(|conn| {
            conn.run_pending_migrations(MIGRATIONS)
                .expect("diesel migrations");
        })
        .await;

    rocket
}

pub fn busses_data() -> AdHoc {
    AdHoc::on_ignite("Data related to busses", |rocket| async {
        rocket
            .attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/busses", routes![list, get_one_bus, delete_one_bus])
    })
}
