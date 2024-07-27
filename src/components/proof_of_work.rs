use data_encoding::HEXLOWER;
use num_bigint::{BigInt, Sign};
use std::ops::ShlAssign;

use crate::components::{blocks::Block, helpers::sha256_digest};

const MAX_NONCE: i64 = i64::MAX;
const TARGET_BITS: i32 = 8;

pub struct ProofOfWork {
    block: Block,
    target: BigInt,
}

impl ProofOfWork {
    pub fn new(block: Block) -> Self {
        let mut target = BigInt::from(1);

        target.shl_assign(256 - TARGET_BITS);

        Self { block, target }
    }

    pub fn prepare_data(&self, nonce: i64) -> Vec<u8> {
        let pre_block_hash = self.block.get_prev_block_hash();
        let transaction_hash = self.block.hash_transactions();
        let timestamp = self.block.get_timestamp();
        let mut data_bytes = vec![];
        data_bytes.extend(pre_block_hash.as_bytes());
        data_bytes.extend(transaction_hash);
        data_bytes.extend(timestamp.to_be_bytes());
        data_bytes.extend(TARGET_BITS.to_be_bytes());
        data_bytes.extend(nonce.to_be_bytes());
        data_bytes
    }

    pub fn run(&self) -> (i64, String) {
        let mut nonce = 0;
        let mut hash = Vec::new();

        println!("Mining the block...");

        while nonce < MAX_NONCE {
            let data = self.prepare_data(nonce);
            // let mut hash = Vec::new();

            hash = sha256_digest(data.as_slice());

            let hash_int = BigInt::from_bytes_be(Sign::Plus, hash.as_slice());

            if hash_int.lt(&self.target) {
                println!("{}", HEXLOWER.encode(hash.as_slice()));
                break;
            } else {
                nonce += 1;
            }
        }
        println!();
        (nonce, HEXLOWER.encode(hash.as_slice()))
    }
}
