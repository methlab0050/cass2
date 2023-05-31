use std::{sync::Arc, fs::read_to_string};

use actix_web::{Responder, web::{Json, Path}, get, HttpRequest, post, HttpServer, App};
use mongodb::{Client, options::{ClientOptions, FindOptions}, error::Result, Database, bson::{doc, oid::ObjectId}};
use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Deserialize, Clone)]
struct Config {
    connection_str: String,
    api_addr: String,
    api_keys: Vec<String>,
}

static mut CONFIG: Option<Config> = None;

fn get_config() -> Config {
    match unsafe{ &CONFIG } {
        Some(conf) => conf.clone(),
        None => {
            let conf = read_to_string("./config.json")
                .expect("could not read or find config.json in root of file");
            serde_json::from_str::<Config>(&conf)
                .expect("config.json not properly formatted")
        },
    }
}

static mut SESSION: Option<Arc<Database>> = None;

async fn get_db_handle() -> Result<Arc<Database>> {
    let db = match unsafe { &SESSION } {
        Some(db) => db.clone(),
        None => {
            let conf = get_config();
            let options = ClientOptions::parse(conf.connection_str).await?;
            let client = Client::with_options(options)?;
            let db = client.database("prod");
            let db = Arc::new(db);
            unsafe { SESSION = Some(db.clone()) };
            db
        },
    };
    Ok(db)
}

fn auth(req: &HttpRequest) -> bool {
    let api_keys = get_config().api_keys;
    let headers = req.headers();
    let get_header = |header_key| {
        match headers.get(header_key).map(|x| x.to_str()) {
            Some(Ok(header_value)) => api_keys.contains(&header_value.to_owned()),
            _ => false,
        }
    };
    get_header("auth") || get_header("authentication")
}

fn get_params(req: &HttpRequest) -> String {
    let mut params = String::new();

    for (name, value) in req.headers() {
        if name.as_str().starts_with("p-") {
            params += &format!("{}|{}\n", &name.as_str()[2..], value.to_str().unwrap_or_default());
        }
    }

    params.to_lowercase()
}

#[derive(Serialize, Deserialize, Debug)]
struct DbRecord {
    _id: ObjectId,
    email: String,
    params: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PartialDbRecord {
    email: String,
    params: Option<String>,
}

#[get("/fetch/{email_type}/{no_of_emails}")]
async fn fetch(path: Path<(String, usize)>, req: HttpRequest) -> impl Responder {
    if !auth(&req) {
        return Json(json!({
            "errors": ["not authenticated"],
        }));
    };
    let db = match get_db_handle().await {
        Ok(db) => db,
        Err(err) => {
            return Json(json!({
                "combos": [],
                "errors": err.to_string(),
            }));
        }
    };
    let collection = db.collection::<DbRecord>(&path.0);
    let find_options = FindOptions::builder().sort(doc! {"_id": 1}).build();
    let mut cursor = match collection.find(doc! {}, find_options).await {
        Ok(cursor) => cursor,
        Err(err) => {
            return Json(json!({
                "combos": [],
                "errors": err.to_string(),
            }));
        }
    };

    let mut combos = Vec::new();
    let mut errors = Vec::new();
    for _index in 0..path.1 {
        if cursor.advance().await.ok() != Some(true) { 
            break; 
        }
        let combo = match cursor.deserialize_current() {
            Ok(combo) => {
                if let Err(err) = collection.delete_one(doc! { "_id": combo._id }, None).await {
                    errors.push(err.to_string())
                };
                combo
            },
            Err(err) => {
                errors.push(err.to_string());
                continue;
            }
        };
        combos.push(combo);
    }

    
    Json(json!({
        "combos": combos,
        "errors": errors,
    }))
}

#[post("/add/{email_type}")]
async fn add(path: Path<String>, body: Json<Vec<String>>, req: HttpRequest) -> impl Responder {
    if !auth(&req) {
        return format!("error:not authenticated");
    };
    
    let db = match get_db_handle().await {
        Ok(db) => db,
        Err(err) => return format!("error:{err}")
    };
    let collection = db.collection::<PartialDbRecord>(&path);
    let params = get_params(&req);
    let emails = body.0.into_iter()
        .map(|email| PartialDbRecord { email, params: Some(params.clone()) });
    
    if let Err(err) = collection.insert_many(emails, None).await {
        format!("error:{err}")
    } else {
        format!("success:all combos added to collection")
    }
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    let addr = &get_config().api_addr;
    HttpServer::new(|| {
            App::new()
                .service(fetch)
                .service(add)
        })
        .bind(addr)?
        .run()
        .await
}
