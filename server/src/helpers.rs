use chrono::prelude::*;
use serde::Deserialize;
use std::{
    error::Error, 
    fmt::{
        Formatter, 
        Display
    }
};
use egg_mode::{
    Token, 
    KeyPair
};

use crate::data_model::Sig;

pub type Void = Result<(), Box<dyn std::error::Error>>;
pub type Res<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server_port: u16,
    pub static_location: String,

    pub with_analyzer: bool,

    pub twitter_consumer_key: String,
    pub twitter_consumer_secret: String,
    pub twitter_access_token: String,
    pub twitter_access_secret: String,

    pub mongo_endpoint: String,

    pub signature_length: usize,
    pub min_shingle_size: usize,
    pub max_shingle_size: usize,
    pub num_shingles_evaluated: usize,

    pub twitter_handles: Vec<String>,
}

pub fn slice_to_u64_le(data: &[u8]) -> u64 {
    assert!(data.len() <= 8, "There must be less than eight (8) bytes for a conversion to u64.");

    let mut result = 0;
    let mut shift = 0;
    for d in data {
        result += (*d as u64) << shift;
        shift += 8;
    }
    
    result
}

pub fn get_time_string(secs: i64) -> String {
    let naive = NaiveDateTime::from_timestamp(secs, 0);
    let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

    format!("{}", datetime.with_timezone(&chrono::Local))
}

pub fn get_twitter_token(config: &Config) -> Token {
    let con_pair = KeyPair::new(config.twitter_consumer_key.clone(), config.twitter_consumer_secret.clone());
    let acc_pair = KeyPair::new(config.twitter_access_token.clone(), config.twitter_access_secret.clone());
    
    Token::Access {
        consumer: con_pair,
        access: acc_pair,
    }
}

pub fn polish_text(s: &str) -> String {
    s.trim().to_lowercase().replace(&['(', ')', ',', '\"', '.', '!', ';', ':', '\'', '“', '”', '’', '&', '?', '‘', '—', '–'][..], "")
}

pub fn compute_similarity_strength(sig1: &Sig, sig2: &Sig) -> f64 {
    assert!(sig1.len() == sig2.len(), "The signatures do not have the same length");

    let signature_length = sig1.len();
    let mut count: usize = 0;

    for k in 0..signature_length {
        if sig1[k].min_hash == sig2[k].min_hash {
            count += 1;
        }
    }

    (count as f64) / (signature_length as f64)
}

pub fn compute_similarity_handles(user_handle1: &str, user_handle2: &str) -> (String, String) {
    let mut handles = [user_handle1, user_handle2];
    handles.sort_unstable();

    (handles[0].to_owned(), handles[1].to_owned())
}

// TODO: If we wanted this to be _really_ slick, then this would not use `split`, and it would just pass
// back slices into the original string.
pub fn get_shingles_up_to_size(text: &str, size: usize) -> Vec<String> {
    let mut result = Vec::<String>::with_capacity(100);
    let splits = text.split_ascii_whitespace().collect::<Vec<&str>>();
    let count = splits.len();

    // Long story short, this iterates through the splits and gets shingles of up to `size`, inclusive.
    for k in 0..count {
        for j in 1..(size+1) {
            if k + j < count {
                let shingle = splits[k..(k+j)]
                    .iter()
                    .fold(String::with_capacity(50), |agg, s| format!("{} {}", agg, s))
                    .trim().to_owned();
                    
                result.push(shingle);
            }
        }
    }

    result
}

#[derive(Debug)]
pub struct GenericError {
    message: String
}

impl From<&str> for GenericError {
    fn from(message: &str) -> Self {
        GenericError { message: message.to_owned() }
    }
}

impl From<String> for GenericError {
    fn from(message: String) -> Self {
        GenericError { message }
    }
}

impl Display for GenericError {
    fn fmt<'a>(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.message);
    }
}

impl Error for GenericError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}