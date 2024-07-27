use std::{
    collections::HashMap,
    env::current_dir,
    sync::{Arc, RwLock},
};

use data_encoding::HEXLOWER;
use sled::{transaction::TransactionResult, Db, Tree};

use super::{
    blocks::Block,
    transaction::{Transaction, TxOutput},
};

const LATEST_BLOCK_HASH: &str = &"latest_block_hash";
const BLOCKS_TREE: &str = "blocks";

#[derive(Debug, Clone)]
pub struct Blockchain {
    latest_blk_hash: Arc<RwLock<String>>,
    db: Db,
}

impl Blockchain {
    pub fn init(genesis_address: &str) -> Self {
        let db = sled::open(current_dir().unwrap().join("data")).unwrap();
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();

        let data = blocks_tree.get(LATEST_BLOCK_HASH).unwrap();
        let latest_blk_hash;

        if data.is_none() {
            let coinbase_tx = Transaction::coinbase_tx(genesis_address);
            let block = Block::generate_genesis_block(&coinbase_tx);
            Self::update_blocks_tree(&blocks_tree, &block);
            latest_blk_hash = String::from(block.get_hash());
        } else {
            latest_blk_hash = String::from_utf8(data.unwrap().to_vec()).unwrap();
        }

        Self {
            latest_blk_hash: Arc::new(RwLock::new(latest_blk_hash)),
            db,
        }
    }

    fn update_blocks_tree(block_tree: &Tree, block: &Block) {
        let block_hash = block.get_hash();
        let _: TransactionResult<(), ()> = block_tree.transaction(|tx_db| {
            let _ = tx_db.insert(block_hash, block.clone());
            let _ = tx_db.insert(LATEST_BLOCK_HASH, block_hash);
            Ok(())
        });
    }

    pub fn new() -> Self {
        let db = sled::open(current_dir().unwrap().join("data")).unwrap();
        let blocks_tree = db.open_tree(BLOCKS_TREE).unwrap();

        let latest_bytes = blocks_tree
            .get(LATEST_BLOCK_HASH)
            .unwrap()
            .expect("No existing blockchain found. Create one.");

        let latest_blk_hash = String::from_utf8(latest_bytes.to_vec()).unwrap();
        Self {
            latest_blk_hash: Arc::new(RwLock::new(latest_blk_hash)),
            db,
        }
    }

    pub fn get_db(&self) -> &Db {
        &self.db
    }

    pub fn get_latest_blk_hash(&self) -> String {
        self.latest_blk_hash.read().unwrap().clone()
    }

    pub fn set_latest_blk_hash(&self, new_latest_blk_hash: &str) {
        let mut latest_blk_hash = self.latest_blk_hash.write().unwrap();
        *latest_blk_hash = String::from(new_latest_blk_hash);
    }

    pub fn mine_block(&self, transactions: &[Transaction]) -> Block {
        for transaction in transactions {
            if transaction.verify(self) == false {
                panic!("Error: Invalid transaction");
            }
        }

        let best_height = self.get_best_height();
        let block = Block::new(self.get_latest_blk_hash(), transactions, best_height + 1);
        let block_hash = block.get_hash();
        let blocks_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        Self::update_blocks_tree(&blocks_tree, &block);

        self.set_latest_blk_hash(block_hash);
        block
    }

    pub fn add_block(&self, block: &Block) {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        if let Some(_) = block_tree.get(block.get_hash()).unwrap() {
            return;
        }

        let _: TransactionResult<(), ()> = block_tree.transaction(|tx_db| {
            let _ = tx_db.insert(block.get_hash(), block.serialize());

            let latest_blk_bytes = tx_db
                .get(self.get_latest_blk_hash())
                .unwrap()
                .expect("The latest hash is not valid");

            let latest_block = Block::deserialize(latest_blk_bytes.as_ref());

            if block.get_height() > latest_block.get_height() {
                let _ = tx_db.insert(LATEST_BLOCK_HASH, block.get_hash()).unwrap();
                self.set_latest_blk_hash(block.get_hash());
            }

            Ok(())
        });
    }

    pub fn get_best_height(&self) -> usize {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        let latest_blk_bytes = block_tree
            .get(self.get_latest_blk_hash())
            .unwrap()
            .expect("The latest hash is valid");
        let latest_block = Block::deserialize(latest_blk_bytes.as_ref());
        latest_block.get_height()
    }

    pub fn iterator(&self) -> BlockchainIterator {
        BlockchainIterator::new(self.get_latest_blk_hash(), self.db.clone())
    }

    pub fn find_utxo(&self) -> HashMap<String, Vec<TxOutput>> {
        let mut utxo: HashMap<String, Vec<TxOutput>> = HashMap::new();
        let mut stxo: HashMap<String, Vec<usize>> = HashMap::new();

        let mut iterator = self.iterator();

        loop {
            let option = iterator.next();
            if option.is_none() {
                break;
            }

            let block = option.unwrap();
            'outer: for tx in block.get_transactions() {
                let txid_hex = HEXLOWER.encode(tx.get_id());
                for (idx, out) in tx.get_vout().iter().enumerate() {
                    if let Some(outs) = stxo.get(txid_hex.as_str()) {
                        for stxo_out_idx in outs {
                            if idx.eq(stxo_out_idx) {
                                continue 'outer;
                            }
                        }
                    }

                    if utxo.contains_key(txid_hex.as_str()) {
                        utxo.get_mut(txid_hex.as_str()).unwrap().push(out.clone());
                    } else {
                        utxo.insert(txid_hex.clone(), vec![out.clone()]);
                    }
                }

                if tx.is_coinbase() {
                    continue;
                }

                for txin in tx.get_vin() {
                    let txid_hex = HEXLOWER.encode(txin.get_txid());
                    if stxo.contains_key(txid_hex.as_str()) {
                        stxo.get_mut(txid_hex.as_str())
                            .unwrap()
                            .push(txin.get_vout());
                    } else {
                        stxo.insert(txid_hex, vec![txin.get_vout()]);
                    }
                }
            }
        }
        utxo
    }

    pub fn find_transaction(&self, txid: &[u8]) -> Option<Transaction> {
        let mut iterator = self.iterator();

        loop {
            let option = iterator.next();

            if option.is_none() {
                break;
            }

            let block = option.unwrap();
            for transaction in block.get_transactions() {
                if txid.eq(transaction.get_id()) {
                    return Some(transaction.clone());
                }
            }
        }
        None
    }

    pub fn get_block_hashes(&self) -> Vec<Vec<u8>> {
        let mut iterator = self.iterator();
        let mut blocks = vec![];
        loop {
            let option = iterator.next();
            if option.is_none() {
                break;
            }
            let block = option.unwrap();
            blocks.push(block.get_hash_bytes());
        }
        blocks
    }

    pub fn get_block(&self, block_hash: &[u8]) -> Option<Block> {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        if let Some(block_bytes) = block_tree.get(block_hash).unwrap() {
            let block = Block::deserialize(block_bytes.as_ref());
            return Some(block);
        }
        None
    }
}

#[derive(Debug)]
pub struct BlockchainIterator {
    db: Db,
    current_hash: String,
}

impl BlockchainIterator {
    fn new(latest_blk_hash: String, db: Db) -> Self {
        Self {
            current_hash: latest_blk_hash,
            db,
        }
    }

    pub fn next(&mut self) -> Option<Block> {
        let block_tree = self.db.open_tree(BLOCKS_TREE).unwrap();
        let data = block_tree.get(self.current_hash.clone()).unwrap();

        if data.is_none() {
            return None;
        }

        let block = Block::deserialize(data.unwrap().to_vec().as_slice());
        self.current_hash = block.get_prev_block_hash().clone();
        Some(block)
    }
}
