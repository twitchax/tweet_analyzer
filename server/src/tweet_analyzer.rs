use tokio::{task, sync::{mpsc}};
use log::{info, error};
use std::collections::HashMap;

use crate::data_model::{SharedClient, Shingle};
use crate::helpers::{self, Void, Config};
use crate::mhs::Mhs;

pub fn start(config: &Config, mongo_client: &SharedClient, mut analyze_tweets_rx: mpsc::UnboundedReceiver<String>, signature_ready_tx: &mpsc::UnboundedSender<String>) {
    let mongo_client_clone = mongo_client.clone();
    let signature_ready_tx_clone = signature_ready_tx.clone();

    let min_shingle_size = config.min_shingle_size;
    let max_shingle_size = config.max_shingle_size;
    let num_shingles_evaluated = config.num_shingles_evaluated;
    let signature_length = config.signature_length;

    let _ = task::spawn(async move {
        while let Some(handle) = analyze_tweets_rx.recv().await {
            let mongo_client_clone2 = mongo_client_clone.clone();
            let signature_ready_tx_clone2 = signature_ready_tx_clone.clone();

            // BUG: Not displaying errors because it is hard to manually drop non-Send values 
            // (which they need to be since the tokio threadpool picks up awaits on new threads).
            // https://rust-lang.github.io/async-book/07_workarounds/04_send_approximation.html

            let _ = task::spawn(async move {
                if update_tweet_shingles_for(&handle, &mongo_client_clone2, max_shingle_size).await.is_err() {
                    error!("[{}] Failed to get or store shingles.", handle);
                } else {
                    if update_signature_for(&handle, &mongo_client_clone2, min_shingle_size, max_shingle_size, num_shingles_evaluated, signature_length).await.is_err() {
                        error!("[{}] Failed to get or store signature.", handle);
                    } else if let Err(e) = signature_ready_tx_clone2.send(handle.to_owned()) {
                        error!("[{}] Failed to send on `signature_ready_tx`: {}", handle, e);
                    }
                }
            });
        }
    });
}

async fn update_tweet_shingles_for(handle: &str, mongo_client: &SharedClient, max_shingle_size: usize) -> Void {
    info!("[{}] Updating shingles.", handle);

    let tweets = mongo_client.get_tweets_for(handle).await?;
    let mut shingle_map: HashMap<String, u32> = HashMap::with_capacity(100000);

    // Get all shingles and their counts.
    for tweet in tweets {
        for shingle in helpers::get_shingles_up_to_size(&tweet.polished_text, max_shingle_size) {
            if let Some(count) = shingle_map.get_mut(&shingle) {
                *count += 1;
            } else {
                shingle_map.insert(shingle, 1);
            }
        }
    }

    // The map was for fast lookup to get aggregates.  Now, convert to vector.
    let shingles = shingle_map.into_iter().map(|(k, v)| {
        let length = k.chars().filter(|c| c == &' ').count() + 1;

        Shingle { text: k, length: length as u32, count: v }
    }).collect::<Vec<Shingle>>();
    
    // Insert shingles into database.
    mongo_client.replace_shingles_for(handle, &shingles).await.unwrap();

    Ok(())
}

async fn update_signature_for(handle: &str, mongo_client: &SharedClient, min_shingle_size: usize, max_shingle_size: usize, num_shingles_evaluated: usize, signature_length: usize) -> Void {
    info!("[{}] Computing signature.", handle);

    let shingles = mongo_client.get_shingles_for(handle, min_shingle_size, max_shingle_size, num_shingles_evaluated).await?;

    let signature = Mhs::new(signature_length).get_signature(shingles.iter().map(|s| &s.text[..]));
    
    mongo_client.replace_signature_for(handle, signature).await?;

    Ok(())
}