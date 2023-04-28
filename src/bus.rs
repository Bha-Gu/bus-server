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
#[diesel(table_name = current_location)]
struct CurrentLocation {
    busid: String,
    latitude: f32,
    longitude: f32
}

table! {
    current_location (busid) {
        busid -> Text,
        latitude -> Float,
        longitude -> Float,
    }
}

#[post("/", data="<post>")]
async fn bus_post(db: Db, post: Json<CurrentLocation>) -> Result<Json<bool>> {
    let post_value = post.clone();
    let a = db.run(move |conn| {
        let a = match current_location::table
            .select(current_location::busid)
            .load::<String>(conn) {
                Ok(a) => a.len() > 0 ,
                Err(_) => false
            };
        if a {
            match diesel::replace_into(current_location::table)
                .values(&*post_value)
                .execute(conn) {
                    Ok(_) => Ok(true),
                    Err(a) => Err(a) 
                }
            }
        else {
            Ok(false)
        }   
    }).await?;
    Ok(Json(a))
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<String>>> {
    let ids: Vec<String> = db.run(move |conn| {
        current_location::table
            .select(current_location::busid)
            .load(conn)
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn get_one_bus(db: Db, id: String) -> Result<Json<CurrentLocation>> {
    let out = db.run(move |conn| {
        current_location::table
            .filter(current_location::busid.eq(id))
            .first(conn)
    }).await.map(Json)?;

    Ok(out)
}

#[delete("/<id>")]
async fn delete_one_bus(db: Db, id: String) -> Result<Option<()>> {
    let out = db.run(move |conn| {
        diesel::delete(current_location::table)
            .filter(current_location::busid.eq(id))
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

pub fn bus_data() -> AdHoc {
    AdHoc::on_ignite("Data related to busses", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/bus", routes![bus_post, list, get_one_bus, delete_one_bus])
    })
}