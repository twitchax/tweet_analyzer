#![warn(rust_2018_idioms, clippy::all)]
#![feature(proc_macro_hygiene, decl_macro)]

// Modules.

mod helpers;
mod data_model;
mod mhs;
mod tweet_grabber;
mod tweet_analyzer;
mod similarity_computer;
mod web;

// Macros.

use log;

// Imports.

use simple_logger;
use toml;
use yansi;
use tokio::sync::mpsc;
use log::{
    info, 
    warn, 
    LevelFilter
};

use data_model::SharedClient;
use helpers::{
    Void, 
    Config
};

// TODO:
//   * Make the shingling, and signature writing, a rayon-based operation.

#[tokio::main]
async fn main() -> Void {
    let args: Vec<String> = std::env::args().collect();
    
    // Ingest config.

    let config = if args.len() == 2 {
        let config_file = args[1].to_owned();
        let config_file_data = tokio::fs::read(config_file).await?;
        let config_text = std::str::from_utf8(&config_file_data)?;

        toml::from_str::<Config>(config_text)?
    } else {
        panic!("This executable requires that a config file be passed in.");
    };

    // Set up logging.

    simple_logger::init().unwrap();
    log::set_max_level(LevelFilter::Info);

    // Kick off analyzer.

    if config.with_analyzer {
        let config_clone = config.clone();

        tokio::task::spawn(async move {
            start_analyzer(&config_clone).await.unwrap();
        });
    }

    // Start rocket server.

    web::start(config).await?;

    Ok(())
}

async fn start_analyzer(config: &Config) -> Void {
    let twitter_token = helpers::get_twitter_token(&config);
    let mongo_client = SharedClient::new(&config.mongo_endpoint).await?;

    // We can make these bounded, if needed.
    let (process_handle_tx, process_handle_rx) = mpsc::unbounded_channel::<String>();
    let (analyze_tweets_tx, analyze_tweets_rx) = mpsc::unbounded_channel::<String>();
    let (signature_ready_tx, signature_ready_rx) = mpsc::unbounded_channel::<String>();
    let (similarities_ready_tx, mut similarities_ready_rx) = mpsc::unbounded_channel::<String>();

    tweet_grabber::start(&twitter_token, &mongo_client, process_handle_rx, &analyze_tweets_tx);
    tweet_analyzer::start(&config, &mongo_client, analyze_tweets_rx, &signature_ready_tx);
    similarity_computer::start(&mongo_client, signature_ready_rx, &similarities_ready_tx);

    for handle in &config.twitter_handles {
        let _ = process_handle_tx.send(handle.to_owned());
    }
    
    while let Some(handle) = similarities_ready_rx.recv().await {
        info!("Similarities ready for {}.", yansi::Paint::green(handle));
    }

    Ok(())
}

