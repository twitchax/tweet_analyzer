use egg_mode::{
    Token, 
    tweet
};
use log::{
    info, 
    error
};
use tokio::{
    task, 
    sync::mpsc, 
    time::Duration
};

use crate::data_model::{
    self, 
    SharedClient
};
use crate::helpers::{
    self, 
    Void
};

pub fn start(
    twitter_token: &Token,
    mongo_client: &SharedClient,
    mut process_handle_rx: mpsc::UnboundedReceiver<String>,
    analyze_tweets_tx: &mpsc::UnboundedSender<String>
) {
    let twitter_token_clone = twitter_token.clone();
    let mongo_client_clone = mongo_client.clone();
    let analyze_tweets_tx_clone = analyze_tweets_tx.clone();
    
    let _ = task::spawn(async move {
        while let Some(handle) = process_handle_rx.recv().await {
            let twitter_token_clone2 = twitter_token_clone.clone();
            let mongo_client_clone2 = mongo_client_clone.clone();
            let analyze_tweets_tx_clone2 = analyze_tweets_tx_clone.clone();
            
            let _ = task::spawn(async move {
                if let Err(e) = get_and_save_tweets_for(&handle[..], &twitter_token_clone2, &mongo_client_clone2).await {
                    error!("[{}] Failed to get or store tweets: {}", handle, e);
                } else if let Err(e) = analyze_tweets_tx_clone2.send(handle.to_owned()) {
                    error!("[{}] Failed to send on `analyze_tweets_tx`: {}", handle, e);
                }
            });
        }
    });
}

async fn get_and_save_tweets_for(
    handle: &str,
    twitter_token: &Token,
    mongo_client: &SharedClient
) -> Void {
    info!("[{}] Getting tweets.", handle);

    // First, see if this handle already has tweets.
    let most_recent_tweet = mongo_client.get_most_recent_tweet_for(handle).await?;
    let min_id = most_recent_tweet.map(|t| t.id);

    // Get the first timeline struct.
    let timeline = tweet::user_timeline(handle.to_owned(), false /* with_replies */, false /* with retweets */, twitter_token).with_page_size(200);
    let mut total_tweets = 0;

    let mut all_tweets = Vec::<data_model::Tweet>::with_capacity(3200);

    let mut max_id = None;

    loop {
        let twitter_result = timeline.call(min_id, max_id).await;

        if let Err(e) = &twitter_result {
            info!("[{}] Twitter API limit reached: waiting 60 seconds: {}.", handle, e);
            tokio::time::delay_for(Duration::from_secs(60)).await;
            continue;
        }

        let response = twitter_result.unwrap();

        let rls = response.rate_limit_status;
        let tweets = response.response;
        let num_tweets = tweets.len();
        total_tweets += num_tweets;

        info!("[{}] Retrieved {} tweets ({} total).  {} / {} calls remaining (refreshing at {}).", handle, num_tweets, total_tweets, rls.remaining, response.rate_limit_status.limit, helpers::get_time_string(response.rate_limit_status.reset as i64));

        if num_tweets == 0 {
            break;
        }

        // Update max_id for the next iteration.
        max_id = tweets.iter().map(|t| t.id).min().map(|n| n - 1);

        // Prepare tweet objects and push into result vector.
        tweets.into_iter().for_each(|t| {
            let u = "Unknown".to_owned();

            let (handle, name, user_id) = if let Some(ref user) = t.user { 
                (&user.screen_name, &user.name, user.id)
            } else { 
                (&u, &u, 0)
            };

            let polished_text = helpers::polish_text(&t.text);

            all_tweets.push(data_model::Tweet {
                id: t.id,
                user_name: name.to_owned(),
                user_handle: handle.to_lowercase(),
                user_id,
                created_at: t.created_at.timestamp(),
                created_at_string: format!("{}", t.created_at.with_timezone(&chrono::Local)),
                text: t.text.to_owned(),
                polished_text
            });
        });
        
    }

    if !all_tweets.is_empty() {
        mongo_client.insert_tweets(&all_tweets).await?;
    }

    Ok(())
}