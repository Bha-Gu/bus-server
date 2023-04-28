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
#[diesel(table_name = busses)]
struct Busses {
    placeid: String,
    busid: String
}

table! {
    busses (placeid) {
        placeid -> Text,
        busid -> Text,
    }
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<String>>> {
    let ids: Vec<String> = db.run(move |conn| {
        busses::table
            .select(busses::placeid)
            .load(conn)
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn get_one_bus(db: Db, id: String) -> Result<Json<Busses>> {
    let out = db.run(move |conn| {
        busses::table
            .filter(busses::placeid.eq(id))
            .first(conn)
    }).await.map(Json)?;

    Ok(out)
}

#[delete("/<id>")]
async fn delete_one_bus(db: Db, id: String) -> Result<Option<()>> {
    let out = db.run(move |conn| {
        diesel::delete(busses::table)
            .filter(busses::busid.eq(id))
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

pub fn busses_data() -> AdHoc {
    AdHoc::on_ignite("Data related to busses", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/busses", routes![list, get_one_bus, delete_one_bus])
    })
}