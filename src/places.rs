use rocket::fairing::AdHoc;
use rocket::response::{status::Created, Debug, Responder};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{http, Build, Rocket};

use self::diesel::prelude::*;
use rocket_sync_db_pools::diesel;

use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions, Method};

#[database("diesel")]
struct Db(diesel::SqliteConnection);

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = place_location)]
struct PlaceLocation {
    busid: String,
    latitude: f32,
    longitude: f32,
}

table! {
    place_location (busid) {
        busid -> Text,
        latitude -> Float,
        longitude -> Float,
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

#[post("/", data = "<post>")]
async fn bus_post<'r, 'o: 'r>(db: Db, post: Json<PlaceLocation>) -> Result<impl Responder<'r, 'o>> {
    let post_value = post.clone();
    db.run(move |conn| {
        diesel::insert_into(place_location::table)
            .values(&*post_value)
            .execute(conn)
    })
    .await?;
    let out = Created::new("/").body(post);
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[get("/")]
async fn list<'r, 'o: 'r>(db: Db) -> Result<impl Responder<'r, 'o>> {
    let ids: Vec<String> = db
        .run(move |conn| {
            place_location::table
                .select(place_location::busid)
                .load(conn)
        })
        .await?;

    let out = Json(ids);
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[get("/all")]
async fn list_all<'r, 'o: 'r>(db: Db) -> Result<impl Responder<'r, 'o>> {
    let ids: Vec<PlaceLocation> = db
        .run(move |conn| {
            place_location::table
                .load::<PlaceLocation>(conn)
        })
        .await?;

    let out = Json(ids);
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[get("/one/<id>")]
async fn get_one_bus<'r, 'o: 'r>(db: Db, id: String) -> Result<impl Responder<'r, 'o>> {
    let out: Json<PlaceLocation> = db
        .run(move |conn| {
            place_location::table
                .filter(place_location::busid.eq(id))
                .first(conn)
        })
        .await
        .map(Json)?;

    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(out)))
}

#[delete("/one/<id>")]
async fn delete_one_bus<'r, 'o: 'r>(db: Db, id: String) -> Result<impl Responder<'r, 'o>> {
    let out: usize = db
        .run(move |conn| {
            diesel::delete(place_location::table)
                .filter(place_location::busid.eq(id))
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

pub fn place_data() -> AdHoc {
    AdHoc::on_ignite("Data related to places", |rocket| async {
        rocket
            .attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount(
                "/place",
                routes![bus_post, list,list_all, get_one_bus, delete_one_bus],
            )
    })
}
