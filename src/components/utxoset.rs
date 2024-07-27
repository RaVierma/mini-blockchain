use std::collections::HashMap;

use data_encoding::HEXLOWER;

use super::{blockchain::Blockchain, blocks::Block, transaction::TxOutput};

const UTXO_TREE: &str = "Chainstate";

pub struct UTXOSet {
    blockchain: Blockchain,
}

impl UTXOSet {
    pub fn new(blockchain: Blockchain) -> Self {
        Self { blockchain }
    }

    pub fn get_blockchain(&self) -> &Blockchain {
        &self.blockchain
    }

    pub fn find_spendable_outputs(
        &self,
        pub_key_hash: &[u8],
        amount: i32,
    ) -> (i32, HashMap<String, Vec<usize>>) {
        let mut unspent_outputs: HashMap<String, Vec<usize>> = HashMap::new();
        let mut accumlated_amount: i32 = 0;
        let db = self.blockchain.get_db();
        let utxo_tree = db.open_tree(UTXO_TREE).unwrap();

        for item in utxo_tree.iter() {
            let (k, v) = item.unwrap();
            let txid_hex = HEXLOWER.encode(k.to_vec().as_slice());
            let outs: Vec<TxOutput> = bincode::deserialize(v.to_vec().as_slice())
                .expect("unable to deserialize TxOutput");

            for (idx, out) in outs.iter().enumerate() {
                if out.is_locked_with_key(pub_key_hash) && accumlated_amount < amount {
                    accumlated_amount += out.get_value();
                    if unspent_outputs.contains_key(txid_hex.as_str()) {
                        unspent_outputs
                            .get_mut(txid_hex.as_str())
                            .unwrap()
                            .push(idx);
                    } else {
                        unspent_outputs.insert(txid_hex.clone(), vec![idx]);
                    }
                }
            }
        }
        (accumlated_amount, unspent_outputs)
    }

    pub fn find_utxo(&self, pub_key_hash: &[u8]) -> Vec<TxOutput> {
        let db = self.blockchain.get_db();
        let utxo_tree = db.open_tree(UTXO_TREE).unwrap();
        let mut utxos = vec![];
        for item in utxo_tree.iter() {
            let (_, v) = item.unwrap();
            let outs: Vec<TxOutput> = bincode::deserialize(v.to_vec().as_slice())
                .expect("unable to deserialize TxOutput");
            for out in outs.iter() {
                if out.is_locked_with_key(pub_key_hash) {
                    utxos.push(out.clone());
                }
            }
        }
        utxos
    }

    pub fn count_transaction(&self) -> i32 {
        let db = self.blockchain.get_db();
        let utxo_tree = db.open_tree(UTXO_TREE).unwrap();
        let mut count = 0;
        for _ in utxo_tree.iter() {
            count += 1;
        }
        count
    }

    pub fn reindex(&self) {
        let db = self.blockchain.get_db();
        let utxo_tree = db.open_tree(UTXO_TREE).unwrap();
        let _ = utxo_tree.clear().unwrap();

        let utxo_map = self.blockchain.find_utxo();
        for (txid_hex, outs) in &utxo_map {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).unwrap();
            let value = bincode::serialize(outs).unwrap();
            let _ = utxo_tree.insert(txid.as_slice(), value).unwrap();
        }
    }

    pub fn update(&self, block: &Block) {
        let db = self.blockchain.get_db();
        let utxo_tree = db.open_tree(UTXO_TREE).unwrap();
        for tx in block.get_transactions() {
            if tx.is_coinbase() == false {
                for vin in tx.get_vin() {
                    let mut updated_outs = vec![];
                    let out_bytes = utxo_tree.get(vin.get_txid()).unwrap().unwrap();
                    let outs: Vec<TxOutput> = bincode::deserialize(&out_bytes)
                        .expect("unable to deserialize the TxOutput");
                    for (idx, out) in outs.iter().enumerate() {
                        if idx != vin.get_vout() {
                            updated_outs.push(out.clone())
                        }
                    }
                    if updated_outs.len() == 0 {
                        let _ = utxo_tree.remove(vin.get_txid()).unwrap();
                    } else {
                        let out_bytes = bincode::serialize(&updated_outs)
                            .expect("unable to serialize TxOutput");
                        utxo_tree.insert(vin.get_txid(), out_bytes).unwrap();
                    }
                }
            }

            let mut new_outputs = vec![];
            for out in tx.get_vout() {
                new_outputs.push(out.clone());
            }

            let out_bytes = bincode::serialize(&new_outputs).expect("unable to serialize TxOutput");
            utxo_tree.insert(tx.get_id(), out_bytes).unwrap();
        }
    }
}
