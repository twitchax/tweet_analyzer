#![warn(rust_2018_idioms, clippy::all)]

// Modules.

mod helpers;
mod data_model;
mod mhs;
mod tweet_grabber;
mod tweet_analyzer;
mod similarity_computer;
mod web;
mod memory_trimmer;
mod types;

// Imports.

use tokio::sync::mpsc;
use log::{
    info, 
    warn, 
    LevelFilter
};

use data_model::SharedClient;
use helpers::{
    Void,
    Res,
    Config
};

// TODO:
//   * Make the shingling, and signature writing, a rayon-based operation.
//   * Example shingle lookup: db.shingles.find({ user_handle: "handle", length: 3 }).sort({ count: -1 }).limit(20).map(o => `[${o.count}] ${o.text}`)

#[tokio::main]
async fn main() -> Void {
    // Set up logging.

    simple_logger::init().unwrap();
    log::set_max_level(LevelFilter::Info);

    // Ingest config.

    let args: Vec<String> = std::env::args().collect();

    let config = if args.len() == 2 {
        let config_file = args[1].to_owned();
        let config_file_data = tokio::fs::read(config_file).await?;
        let config_text = std::str::from_utf8(&config_file_data)?;

        toml::from_str::<Config>(config_text)?
    } else {
        panic!("This executable requires that a config file be passed in.");
    };

    // Wait for database to come up.

    let mongo_client = SharedClient::new(&config.mongo_endpoint).await?;
    info!("Waiting for database ...");
    mongo_client.wait_for_ready().await;
    info!("Ready!");

    // Kick off analyzer.

    let process_handle_tx = start_analyzer(&config, &mongo_client)?;

    // Start rocket server.

    web::start(&config, &mongo_client, &process_handle_tx).await?;

    Ok(())
}

fn start_analyzer(config: &Config, mongo_client: &SharedClient) -> Res<types::HandleSender> {
    let twitter_token = helpers::get_twitter_token(&config);
    
    // We can make these bounded, if needed.
    let (process_handle_tx, process_handle_rx) = mpsc::unbounded_channel::<String>();
    let (analyze_tweets_tx, analyze_tweets_rx) = mpsc::unbounded_channel::<String>();
    let (signature_ready_tx, signature_ready_rx) = mpsc::unbounded_channel::<String>();
    let (similarities_ready_tx, mut similarities_ready_rx) = mpsc::unbounded_channel::<String>();

    tweet_grabber::start(&twitter_token, &mongo_client, process_handle_rx, &analyze_tweets_tx);
    tweet_analyzer::start(&config, &mongo_client, analyze_tweets_rx, &signature_ready_tx);
    similarity_computer::start(&mongo_client, signature_ready_rx, &similarities_ready_tx);

    // Run a memory trimmer on linux.
    #[cfg(target_os = "linux")]
    memory_trimmer::start();

    if let Some(handles) = &config.twitter_handles {
        for handle in handles {
            let _ = process_handle_tx.send(handle.to_owned());
        }
    }
    
    tokio::task::spawn(async move {
        while let Some(handle) = similarities_ready_rx.recv().await {
            info!("Similarities ready for {}.", yansi::Paint::green(handle));
        }
    });

    Ok(process_handle_tx)
}

