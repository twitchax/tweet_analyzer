use futures::stream::StreamExt;
use serde::Serialize;
use mongodb::{
    Client, 
    Cursor, 
    options::{
        FindOptions,
        FindOneOptions
    }
};
use bson::{
    doc, 
    bson, 
    Document, 
    Bson
};

use crate::helpers::{
    Res,
    Void,
    GenericError
};

static DATABASE_NAME: &str = "tweet_analyzer";

#[derive(Serialize)]
pub struct Shingle {
    pub text: String,
    pub length: u32,
    pub count: u32
}

#[derive(Serialize)]
pub struct Tweet {
    pub id: u64,
    pub user_name: String,
    pub user_handle: String,
    pub user_id: u64,
    pub created_at: i64,
    pub created_at_string: String,
    pub text: String,
    pub polished_text: String
}

#[derive(Serialize)]
pub struct SigEntry {
    pub shingle: String,
    pub min_hash: u64
}

pub type Sig = Vec<SigEntry>;

#[derive(Serialize)]
pub struct Signature {
    pub user_handle: String,
    pub signature: Sig
}

#[derive(Serialize)]
pub struct Similarity {
    pub source_handle: String,
    pub target_handle: String,
    pub strength: f64
}

// The mongo client is `Clone` because it's underlying implementation uses `Arc`, so we can use `Clone`, as well.
#[derive(Clone)]
pub struct SharedClient {
    client: Client
}

impl SharedClient {
    pub async fn new(endpoint: &str) -> Res<Self> {
        let client = Client::with_uri_str(endpoint).await?;

        Ok(Self { client })
    }

    pub async fn wait_for_ready(&self) {
        loop {
            if self.client.list_database_names(None, None).await.is_ok() {
                break;
            } else {
                tokio::time::delay_for(tokio::time::Duration::from_secs(1)).await;
            }
        }
    }

    pub async fn insert_tweets<'a, I>(&self, tweets: I) -> Void
        where I: IntoIterator<Item = &'a Tweet> 
    {
        let documents = tweets.into_iter().map(|t|
            doc! {
                "_id": t.id,
                "user_name": &t.user_name,
                "user_handle": &t.user_handle.to_lowercase(),
                "user_id": t.user_id,
                "created_at": t.created_at,
                "created_at_string": &t.created_at_string, 
                "text": &t.text,
                "polished_text": &t.polished_text
            }
        );

        self.client.database(DATABASE_NAME).collection("tweets").insert_many(documents, None).await?;

        Ok(())
    }

    pub async fn replace_shingles_for<I>(&self, user_handle: &str, shingles: I) -> Void
        where I: IntoIterator<Item = Shingle>
    {
        let documents: Vec<Document> = shingles.into_iter().map(|s| {
            doc! {
                "user_handle": user_handle.to_lowercase(),
                "text": &s.text,
                "length": s.length,
                "count": s.count
            }
        }).collect();

        // TODO: The mongo rust driver does not support transactions yet:
        // should use that when support is ready.

        // Remove old shingles.
        let delete_filter = doc! { "user_handle": user_handle.to_lowercase() };
        self.client.database(DATABASE_NAME).collection("shingles").delete_many(delete_filter, None).await?;

        // Insert in batches.
        for chunk in documents.chunks(10_000usize) {
            self.client.database(DATABASE_NAME).collection("shingles").insert_many(Vec::from(chunk), None).await?;
        }
        
        Ok(())
    }

    pub async fn replace_signature_for(&self, user_handle: &str, signature: Sig) -> Void {
        let signature = doc! {
            "user_handle": user_handle.to_lowercase(),
            "signature": Bson::Array(signature.into_iter().map(|e| bson!({ "shingle": e.shingle, "min_hash": e.min_hash })).collect())
        };

        // TODO: The mongo rust driver does not support transactions yet:
        // should use that when support is ready.

        // Remove old signature.
        let delete_filter = doc! { "user_handle": user_handle.to_lowercase() };
        self.client.database(DATABASE_NAME).collection("signatures").delete_one(delete_filter, None).await?;

        // Insert new signature.
        self.client.database(DATABASE_NAME).collection("signatures").insert_one(signature, None).await?;

        Ok(())
    }

    // TODO: This should really be "replace".  Pretty much useless without transaction support, at the moment.
    pub async fn insert_similarities<I>(&self, similarities: I) -> Void
        where I: IntoIterator<Item = Similarity>
    {
        let documents = similarities.into_iter().map(|s|
            doc! {
                "source_handle": &s.source_handle,
                "target_handle": &s.target_handle,
                "strength": s.strength
            }
        );

        self.client.database(DATABASE_NAME).collection("similarities").insert_many(documents, None).await?;

        Ok(())
    }

    pub async fn get_tweets_for(&self, user_handle: &str) -> Res<Vec<Tweet>> {
        let filter = doc! { "user_handle": user_handle.to_lowercase() };
        let mut cursor: Cursor = self.client.database(DATABASE_NAME).collection("tweets").find(filter, None).await?;

        let mut result = Vec::<Tweet>::with_capacity(3200 /* max amount of tweets for a user anyway */);

        while let Some(document) = cursor.next().await {
            if let Ok(doc) = document {
                let id = doc.get("_id").and_then(Bson::as_i64).unwrap_or(0) as u64;
                let user_name = doc.get("user_name").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let user_handle = doc.get("user_handle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let user_id = doc.get("user_id").and_then(Bson::as_i64).unwrap_or(0) as u64;
                let created_at = doc.get("created_at").and_then(Bson::as_i64).unwrap_or(0);
                let created_at_string = doc.get("created_at_string").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let text = doc.get("text").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let polished_text = doc.get("polished_text").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();

                result.push(Tweet { id, user_name, user_handle, user_id, created_at, created_at_string, text, polished_text });
            }
        }

        Ok(result)
    }

    pub async fn get_most_recent_tweet_for(&self, user_handle: &str) -> Res<Option<Tweet>> {
        let filter = doc! { "user_handle": user_handle.to_lowercase() };
        let options = FindOneOptions::builder().sort(doc! { "_id": -1 }).build();

        let document = self.client.database(DATABASE_NAME).collection("tweets").find_one(filter, options).await?;

        if let Some(doc) = document {
            let id = doc.get("_id").and_then(Bson::as_i64).unwrap_or(0) as u64;
            let user_name = doc.get("user_name").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
            let user_handle = doc.get("user_handle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
            let user_id = doc.get("user_id").and_then(Bson::as_i64).unwrap_or(0) as u64;
            let created_at = doc.get("created_at").and_then(Bson::as_i64).unwrap_or(0);
            let created_at_string = doc.get("created_at_string").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
            let text = doc.get("text").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
            let polished_text = doc.get("polished_text").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();

            return Ok(Some(Tweet { id, user_name, user_handle, user_id, created_at, created_at_string, text, polished_text }));
        }

        Ok(None)
    }

    pub async fn get_shingles_for(&self, user_handle: &str, min_length: usize, max_length: usize, limit: usize) -> Res<Vec<Shingle>> {
        let filter = doc! { "user_handle": user_handle.to_lowercase(), "length": { "$lt": (max_length + 1) as u32, "$gt": (min_length - 1) as u32 } };
        let options = FindOptions::builder().sort(doc! { "count": -1, "text": 1 }).limit(limit as i64).build();

        let mut cursor: Cursor = self.client.database(DATABASE_NAME).collection("shingles").find(filter, options).await?;

        let mut result = Vec::<Shingle>::with_capacity(100000);

        while let Some(document) = cursor.next().await {
            if let Ok(doc) = document {
                let text = doc.get("text").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let length = doc.get("length").and_then(Bson::as_i32).unwrap_or(0) as u32;
                let count = doc.get("count").and_then(Bson::as_i32).unwrap_or(0) as u32;

                result.push(Shingle { text, length, count });
            }
        }

        Ok(result)
    }

    pub async fn _get_signature_for(&self, user_handle: &str) -> Res<Sig> {
        let filter = doc! { "user_handle": user_handle.to_lowercase() };

        let doc: Option<Document> = self.client.database(DATABASE_NAME).collection("signatures").find_one(filter, None).await?;

        if doc.is_none() {
            return Err(Box::new(GenericError::from(format!("Could not find signature for handle {}.", user_handle))));
        }

        let unwrapped_doc = doc.unwrap();

        let signature_array: &bson::Array = unwrapped_doc.get("signature").and_then(Bson::as_array).unwrap();

        let result: Sig = signature_array.iter().map(|entry| {
            let e = entry.as_document().unwrap();
            let shingle = e.get("shingle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
            let min_hash = e.get("min_hash").and_then(Bson::as_i64).unwrap_or(0) as u64;

            SigEntry { shingle, min_hash }
        }).collect();

        Ok(result)
    }

    pub async fn get_all_signatures(&self) -> Res<Vec<Signature>> {
        let mut cursor: Cursor = self.client.database(DATABASE_NAME).collection("signatures").find(None, None).await?;

        let mut result = Vec::<Signature>::with_capacity(100);

        while let Some(document) = cursor.next().await {
            if let Ok(doc) = document {
                let user_handle = doc.get("user_handle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let signature_array: &bson::Array = doc.get("signature").and_then(Bson::as_array).unwrap();

                let sig: Sig = signature_array.iter().map(|entry| {
                    let e = entry.as_document().unwrap();
                    let shingle = e.get("shingle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                    let min_hash = e.get("min_hash").and_then(Bson::as_i64).unwrap_or(0) as u64;

                    SigEntry { shingle, min_hash }
                }).collect();

                result.push(Signature { user_handle, signature: sig });
            }
        }

        Ok(result)
    }

    pub async fn get_all_similarities(&self) -> Res<Vec<Similarity>> {
        let mut cursor: Cursor = self.client.database(DATABASE_NAME).collection("similarities").find(None, None).await?;

        let mut result = Vec::<Similarity>::with_capacity(100);

        while let Some(document) = cursor.next().await {
            if let Ok(doc) = document {
                let source_handle = doc.get("source_handle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let target_handle = doc.get("target_handle").and_then(Bson::as_str).unwrap_or("Unknown").to_owned();
                let strength = doc.get("strength").and_then(Bson::as_f64).unwrap_or(0f64);

                result.push(Similarity { source_handle, target_handle, strength });
            }
        }

        Ok(result)
    }

    // TODO: Make a "get all similarities" and "get specific similarity" function.
}