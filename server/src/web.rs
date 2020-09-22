use rocket::{
    get, 
    post,
    routes, 
    State,
    //Request, Data, Response,
    response::status::{Accepted, NotFound},
    //fairing::{Fairing, Info, Kind}
};
use rocket_contrib::{
    json::Json,
    serve::StaticFiles
};
use log::info;
//use std::sync::atomic::{AtomicUsize, Ordering};

use crate::data_model::{
    Similarity,
    Signature,
    Shingle,
    Tweet,
    SharedClient
};
use crate::helpers::{
    Void,
    Config
};
use crate::types::HandleSender;

struct RocketState {
    mongo_client: SharedClient,
    process_handle_tx: HandleSender,
    config: Config
}

pub async fn start(config: &Config, mongo_client: &SharedClient, process_handle_tx: &HandleSender) -> Void {
    let static_location = config.static_location.to_owned();
    let rocket_config = rocket::config::Config::build(rocket::config::Environment::Staging)
        .port(config.server_port)
        .finalize().unwrap();

    let state = RocketState { config: config.clone(), mongo_client: mongo_client.clone(), process_handle_tx: process_handle_tx.clone() };

    rocket::custom(rocket_config)
        .manage(state)
        //.attach(Telemetry::new())
        .mount("/", StaticFiles::from(static_location))
        .mount("/api", routes![get_similarities, get_signatures, get_shingles, get_tweets, add_handle])
        .launch().await?;

        Ok(())
}

#[get("/similarities")]
async fn get_similarities(state: State<'_, RocketState>) -> Json<Vec<Similarity>> {
    info!("Processing rocket request for similarities ...");
    Json(state.mongo_client.get_all_similarities().await.unwrap())
}

#[get("/signatures")]
async fn get_signatures(state: State<'_, RocketState>) -> Json<Vec<Signature>> {
    info!("Processing rocket request for signatures ...");
    Json(state.mongo_client.get_all_signatures().await.unwrap())
}

#[get("/shingles/<handle>?<min_length>&<max_length>&<limit>")]
async fn get_shingles(handle: String, min_length: Option<usize>, max_length: Option<usize>, limit: Option<usize>, state: State<'_, RocketState>) -> Json<Vec<Shingle>> {
    info!("Processing rocket request for shingles ...");
    
    let min = min_length.unwrap_or(state.config.min_shingle_size);
    let max = max_length.unwrap_or(state.config.max_shingle_size);
    let lim = limit.unwrap_or(state.config.num_shingles_evaluated);

    Json(state.mongo_client.get_shingles_for(&handle, min, max, lim).await.unwrap())
}

#[get("/tweets/<handle>")]
async fn get_tweets(handle: String, state: State<'_, RocketState>) -> Json<Vec<Tweet>> {
    info!("Processing rocket request for shingles ...");

    Json(state.mongo_client.get_tweets_for(&handle).await.unwrap())
}

#[post("/handles/<handle>")]
async fn add_handle(handle: String, state: State<'_, RocketState>) -> Result<Accepted<String>, NotFound<String>> {
    info!("Processing rocket request to add handle ({}) ...", handle);

    // TODO: Not found is wrong here, but my intellisense is broken, and I am lazy.
    state.process_handle_tx.send(handle.clone()).map_err(|e| NotFound(e.to_string()))?;
    
    Ok(Accepted(Some(handle)))
}

// struct Telemetry {
//     count: AtomicUsize
// }

// impl Telemetry {
//     fn new() -> Self {
//         Self { count: AtomicUsize::new(0) }
//     }
// }

// impl Fairing for Telemetry {
//     fn info(&self) -> Info {
//         Info {
//             name: "Telemetry",
//             kind: Kind::Request
//         }
//     }

//     // Increment the counter for `GET` and `POST` requests.
//     fn on_request(&self, request: &mut Request, _: &Data) {
//         info!("Request received ({} total)!", self.count.fetch_add(1, Ordering::Relaxed));
//     }
// }