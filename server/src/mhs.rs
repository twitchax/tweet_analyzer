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

    pub fn get_signature<'a, I>(&self, shingles: I) -> Sig where I: IntoIterator<Item = &'a str> {
        let length = self.randoms.len();
        let mut signature = Sig::with_capacity(length);

        // Fill the signature with max values.
        for _ in 0..length {
            // TODO: This is bad, since we are creating a bunch of empty strings for no reason.  We can make this an option maybe?
            // Or, refactor with code.
            // Yeah, there is another copy down below: we need to be smarter.... :(
            signature.push(SigEntry { shingle: "".to_owned(), min_hash: u64::MAX });
        }

        // Iterate over the shingles on the outside since we can only iterate once.
        // This could be parallelized if needed for efficiency?
        for shingle in shingles {
            for k in 0..length {
                let hash = hash(shingle, self.randoms[k]);

                if hash == 0 {
                    println!("'{}'", shingle.len());
                }
    
                if hash < signature[k].min_hash {
                    signature[k] = SigEntry { shingle: shingle.to_owned() , min_hash: hash };
                }
            }
        }
        
        signature
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