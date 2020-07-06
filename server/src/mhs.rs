use rand::{
    rngs::StdRng, 
    SeedableRng, 
    RngCore
};

use crate::helpers::slice_to_u64_le;
use crate::data_model::{
    Sig, 
    SigEntry
};

static SEED: u64 = 42;

pub struct Mhs {
    randoms: Vec<u64>
}

impl Mhs {
    pub fn new(size: usize) -> Self {
        let mut rand = StdRng::seed_from_u64(SEED);
        let mut randoms = Vec::<u64>::with_capacity(size);

        for _ in 0..size {
            loop {
                let num = rand.next_u64();

                if num % 2 == 1 {
                    randoms.push(num);
                    break;
                }
            }
        }

        Self { randoms }
    }

    /// This takes a Vec since we must iterate over it multiple times.
    /// Might as well allow the caller to specify the best way to get a Vec.
    pub fn get_signature<'a>(&self, shingles: &Vec<&'a str>) -> Sig
    {
        let length = self.randoms.len();

        // Iterate over each hash function and keep the minhash across all shingles.
        (0..length).into_iter().map(|k| {
            let mut min_hash = u64::MAX;
            let mut min_shingle = shingles[0];

            for shingle in shingles {
                let hash = hash(*shingle, self.randoms[k]);

                if hash < min_hash {
                    min_hash = hash;
                    min_shingle = shingle;
                }
            }

            SigEntry { shingle: min_shingle.to_owned() , min_hash }
        }).collect()
    }
}

fn hash(s: &str, a: u64) -> u64 {
    let bytes = s.as_bytes();
    let length = bytes.len();
    let mut result = a;

    for k in (0..length).step_by(8) {
        let num: u64;

        if k + 8 < length {
            num = slice_to_u64_le(&bytes[k..(k+8)])
        } else {
            num = slice_to_u64_le(&bytes[k..length])
        }

        result = result.wrapping_mul(num);
    }

    result
}