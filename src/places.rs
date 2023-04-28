use rocket::{Rocket, Build};
use rocket::fairing::AdHoc;
use rocket::response::{Debug, status::Created};
use rocket::serde::{Serialize, Deserialize, json::Json};

use rocket_sync_db_pools::diesel;

use self::diesel::prelude::*;

#[database("diesel")]
struct Db(diesel::SqliteConnection);

type Result<T, E = Debug<diesel::result::Error>> = std::result::Result<T, E>;

#[derive(Debug, Clone, Deserialize, Serialize, Queryable, Insertable)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = place_location)]
struct PlaceLocation {
    busid: String,
    latitude: f32,
    longitude: f32
}

table! {
    place_location (busid) {
        busid -> Text,
        latitude -> Float,
        longitude -> Float,
    }
}

#[post("/", data="<post>")]
async fn bus_post(db: Db, post: Json<PlaceLocation>) -> Result<Created<Json<PlaceLocation>>> {
    let post_value = post.clone();
    db.run(move |conn| {
        diesel::insert_into(place_location::table)
        .values(&*post_value)
        .execute(conn)
    }).await?;
    Ok(Created::new("/").body(post))
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<String>>> {
    let ids: Vec<String> = db.run(move |conn| {
        place_location::table
            .select(place_location::busid)
            .load(conn)
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn get_one_bus(db: Db, id: String) -> Result<Json<PlaceLocation>> {
    let out = db.run(move |conn| {
        place_location::table
            .filter(place_location::busid.eq(id))
            .first(conn)
    }).await.map(Json)?;

    Ok(out)
}

#[delete("/<id>")]
async fn delete_one_bus(db: Db, id: String) -> Result<Option<()>> {
    let out = db.run(move |conn| {
        diesel::delete(place_location::table)
            .filter(place_location::busid.eq(id))
            .execute(conn)
    }).await?;

    Ok((out == 1).then(|| ()))
}


async fn run_migrations(rocket: Rocket<Build>) -> Rocket<Build> {
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    Db::get_one(&rocket).await
        .expect("database connection")
        .run(|conn| { conn.run_pending_migrations(MIGRATIONS).expect("diesel migrations"); })
        .await;

    rocket
}

pub fn place_data() -> AdHoc {
    AdHoc::on_ignite("Data related to places", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/place", routes![bus_post, list, get_one_bus, delete_one_bus])
    })
}