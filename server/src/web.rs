use rocket::{
    get, 
    routes, 
    State
};
use rocket_contrib::{
    json::Json,
    serve::StaticFiles
};

use crate::data_model::{
    Similarity,
    SharedClient
};
use crate::helpers::{
    Void,
    Config
};

struct RocketState {
    mongo_client: SharedClient
}

pub async fn start(config: Config, mongo_client: SharedClient) -> Void {
    let static_location = config.static_location.to_owned();
    let rocket_config = rocket::config::Config::build(rocket::config::Environment::Staging)
        .port(config.server_port)
        .finalize().unwrap();

    let state = RocketState { mongo_client };

    rocket::custom(rocket_config)
        .manage(state)
        .mount("/", StaticFiles::from(static_location))
        .mount("/api", routes![similarities])
        .launch().await?;

        Ok(())
}

#[get("/similarities")]
async fn similarities(state: State<'_, RocketState>) -> Json<Vec<Similarity>> {
    Json(state.mongo_client.get_all_similarities().await.unwrap())
}