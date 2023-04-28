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
#[diesel(table_name = routes)]
struct Routes {
    busid: String,
    placeid: String
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Routing {
    busid: String,
    places: Vec<(String, f32, f32)>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct RoutesIn {
    busid: String,
    placeid: String,
    latitude: f32,
    longitude: f32
}


// Shimla || Sundernager || Ner Chock || Mandi
table! {
    routes (busid) {
        busid -> Text,
        placeid -> Text,
    }
}

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
async fn bus_post(db: Db, post: Json<RoutesIn>) -> Result<Created<Json<RoutesIn>>> {
    let post_value = post.clone();
    let loc_value = CurrentLocation {
        busid: post_value.busid.clone(),
        latitude: post_value.latitude,
        longitude: post_value.longitude,
    };
    let post_value =  Routes {
        busid: post_value.busid.clone(),
        placeid: post_value.placeid.clone(),
    };
    db.run(move |conn| {
        let post_double = post_value.clone();
        let busid = post_double.busid;
        let places = post_double.placeid;
        let place = places
            .split("|")
            .map(|p| p.to_owned());
        'placing: for i in place {
            if let Ok(place_entry) = busses::table
                .filter(busses::placeid.eq(i.clone()))
                .first::<Busses>(conn) {
                    let bus_list: Busses = place_entry;
                    let bus_exist = bus_list.busid.split("|").any(|bus| bus == busid);
                    if bus_exist {
                        continue 'placing;
                    }
                    else {
                        let busid = format!("{}|{}", bus_list.busid, busid);
                        let a = Busses { placeid: i, busid };
                        diesel::replace_into(busses::table).values(a).execute(conn)?;
                    }
                }
                else {
                    let a = Busses {
                        placeid: i,
                        busid: busid.to_owned()
                    };
                    diesel::insert_into(busses::table).values(a).execute(conn)?;
                }
        }
        match diesel::replace_into(routes::table)
            .values(post_value)
            .execute(conn) {
                Ok(_) => {diesel::replace_into(current_location::table)
                    .values(loc_value)
                    .execute(conn)},
                Err(a) => Err(a)
            }
    }).await?;
    Ok(Created::new("/").body(post))
    
}

#[get("/")]
async fn list(db: Db) -> Result<Json<Vec<String>>> {
    let ids: Vec<String> = db.run(move |conn| {
        routes::table
            .select(routes::busid)
            .load(conn)
    }).await?;

    Ok(Json(ids))
}

#[get("/<id>")]
async fn get_one_bus(db: Db, id: String) -> Result<Json<Routing>> {
    let outs = db.run(move |conn| {
        let out: Routes = routes::table
            .filter(routes::busid.eq(id))
            .first(conn).unwrap();
        let route_bus = out.placeid;
        let a: Vec<String> = route_bus
            .split("|")
            .map(|a| 
                a.to_owned()
            )
            .collect();
        let mut a2: Vec<(String, f32, f32)> = vec![];
        for i in a {
            let out2: PlaceLocation =  
                match place_location::table
                    .filter(place_location::busid.eq(i))
                    .first(conn) {
                        Ok(a) => a,
                        Err(_) => continue,
                    } ;
            a2.push((out2.busid, out2.latitude, out2.longitude));
            // a2.push( (i.to_owned(), 0.0, 0.0) );
        }
        Routing {
            busid: out.busid,
            places: a2
        }
    }).await;
    
    Ok(Json(outs))
}

#[delete("/<id>")]
async fn delete_one_bus(db: Db, id: String) -> Result<Option<()>> {
    let out = db.run(move |conn| {
        diesel::delete(routes::table)
            .filter(routes::busid.eq(id))
            .execute(conn)
    }).await?;

    Ok((out == 1).then(|| ()))
}

#[delete("/")]
async fn destroy(db: Db) -> Result<()> {
    db.run(move |conn| diesel::delete(routes::table).execute(conn)).await?;

    Ok(())
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

pub fn route_data() -> AdHoc {
    AdHoc::on_ignite("Data related to routes", |rocket| async {
        rocket.attach(Db::fairing())
            .attach(AdHoc::on_ignite("Diesel Migrations", run_migrations))
            .mount("/routes", routes![bus_post, list, get_one_bus, delete_one_bus, destroy])
    })
}