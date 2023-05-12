use rocket::fairing::AdHoc;
use rocket::response::{Debug, Responder};
use rocket::serde::{json::Json, Deserialize, Serialize};
use rocket::{http, Build, Rocket};

use rocket_cors::{AllowedHeaders, AllowedOrigins, CorsOptions, Method};
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
    longitude: f32,
}

table! {
    current_location (busid) {
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
async fn bus_post<'r, 'o: 'r>(
    db: Db,
    post: Json<CurrentLocation>,
) -> Result<impl Responder<'r, 'o>> {
    let post_value = post.clone();
    let a: bool = db
        .run(move |conn| {
            let b = &*post_value.clone().busid;
            let a = match current_location::table
                .select(current_location::busid)
                .load::<String>(conn)
            {
                Ok(a) => a.contains(&b.to_string()),
                Err(_) => false,
            };
            if a {
                match diesel::replace_into(current_location::table)
                    .values(&*post_value)
                    .execute(conn)
                {
                    Ok(_) => Ok(true),
                    Err(a) => Err(a),
                }
            } else {
                Ok(false)
            }
        })
        .await?;
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(Json(a))))
}

#[get("/")]
async fn list<'r, 'o: 'r>(db: Db) -> Result<impl Responder<'r, 'o>> {
    let ids: Vec<String> = db
        .run(move |conn| {
            current_location::table
                .select(current_location::busid)
                .load(conn)
        })
        .await?;
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(Json(ids))))
}

#[get("/all")]
async fn list_all<'r, 'o: 'r>(db: Db) -> Result<impl Responder<'r, 'o>> {
    let ids = db
        .run(move |conn| {
            current_location::table
                .load::<CurrentLocation>(conn)
        })
        .await?;
    let options = match core_options().to_cors() {
        Ok(a) => a,
        Err(a) => return Ok(Err(a)),
    };
    Ok(options.respond_owned(move |guard| guard.responder(Json(ids))))
}


#[get("/one/<id>")]
async fn get_one_bus<'r, 'o: 'r>(db: Db, id: String) -> Result<impl Responder<'r, 'o>> {
    let out: Json<CurrentLocation> = db
        .run(move |conn| {
            current_location::table
                .filter(current_location::busid.eq(id))
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
            diesel::delete(current_location::table)
                .filter(current_location::busid.eq(id))
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

pub fn bus_data() -> AdHoc {
    AdHoc::on_ignite("Data related to busses", |rocket| async {
        rocket
            .attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/bus", routes![bus_post, list, list_all, get_one_bus, delete_one_bus])
    })
}
