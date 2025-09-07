use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

#[derive(Serialize, Deserialize, Debug)]
pub struct BloomFilter {
    bitmap: Vec<u8>,
    num_hashes: u32,
}

impl BloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let num_bits = Self::calculate_bits(expected_items, fp_rate);
        let num_hashes = Self::calculate_hashes(num_bits, expected_items);
        BloomFilter {
            bitmap: vec![0; (num_bits + 7) / 8],
            num_hashes,
        }
    }

    // CORRECTED: Add `+ ?Sized` to allow both `&String` and `&str`
    pub fn add<T: Hash + ?Sized>(&mut self, item: &T) {
        for i in 0..self.num_hashes {
            let index = self.get_index(item, i);
            let byte_index = index / 8;
            let bit_index = index % 8;
            self.bitmap[byte_index] |= 1 << bit_index;
        }
    }

    // CORRECTED: Add `+ ?Sized` here as well
    pub fn contains<T: Hash + ?Sized>(&self, item: &T) -> bool {
        for i in 0..self.num_hashes {
            let index = self.get_index(item, i);
            let byte_index = index / 8;
            let bit_index = index % 8;
            if (self.bitmap[byte_index] >> bit_index) & 1 == 0 {
                return false;
            }
        }
        true
    }
    
    // CORRECTED: Add `+ ?Sized` to the helper function too
    fn get_index<T: Hash + ?Sized>(&self, item: &T, seed: u32) -> usize {
        let mut hasher1 = DefaultHasher::new();
        item.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        (hash1 as u32 + seed).hash(&mut hasher2);
        let hash2 = hasher2.finish();
        
        (hash1.wrapping_add((seed as u64).wrapping_mul(hash2))) as usize % (self.bitmap.len() * 8)
    }
    
    fn calculate_bits(n: usize, p: f64) -> usize {
        (-(n as f64 * p.ln()) / (2.0f64.ln().powi(2))).ceil() as usize
    }

    fn calculate_hashes(m: usize, n: usize) -> u32 {
        ((m as f64 / n as f64) * 2.0f64.ln()).ceil() as u32
    }
}