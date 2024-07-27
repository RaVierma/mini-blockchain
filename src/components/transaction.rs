use data_encoding::HEXLOWER;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    blockchain::Blockchain,
    helpers::{
        base58_decode, ecdsa_p256_sha256_sign_digest, ecdsa_p256_sha256_sign_verify, sha256_digest,
    },
    utxoset::UTXOSet,
    wallets::{self, Wallets},
};

const INCENTIVE: i32 = 10;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TxInput {
    txid: Vec<u8>,
    vout: usize,
    signature: Vec<u8>,
    pub_key: Vec<u8>,
}

impl TxInput {
    pub fn new(txid: &[u8], vout: usize) -> Self {
        Self {
            txid: txid.to_vec(),
            vout,
            signature: vec![],
            pub_key: vec![],
        }
    }

    pub fn get_txid(&self) -> &[u8] {
        self.txid.as_slice()
    }

    pub fn get_vout(&self) -> usize {
        self.vout
    }

    pub fn get_pub_key(&self) -> &[u8] {
        self.pub_key.as_slice()
    }

    pub fn uses_key(&self, pub_key_hash: &[u8]) -> bool {
        let locking_hash = wallets::hash_pub_key(self.pub_key.as_slice());
        return locking_hash.eq(pub_key_hash);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxOutput {
    value: i32,
    pub_key_hash: Vec<u8>,
}

impl TxOutput {
    pub fn new(value: i32, address: &str) -> Self {
        let mut out = Self {
            value,
            pub_key_hash: vec![],
        };

        out.lock(address);
        out
    }

    pub fn get_value(&self) -> i32 {
        self.value
    }

    pub fn get_pub_key_hash(&self) -> &[u8] {
        self.pub_key_hash.as_slice()
    }

    fn lock(&mut self, address: &str) {
        let pyaload = base58_decode(address);
        let pub_key_hash = pyaload[1..pyaload.len() - wallets::ADDRESS_CHECK_SUM_LEN].to_vec();
        self.pub_key_hash = pub_key_hash;
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        self.pub_key_hash.eq(pub_key_hash)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    id: Vec<u8>,
    vin: Vec<TxInput>,
    vout: Vec<TxOutput>,
}

impl Transaction {
    pub fn coinbase_tx(to: &str) -> Self {
        let txout = TxOutput::new(INCENTIVE, to);
        let mut txinput = TxInput::default();
        txinput.signature = Uuid::new_v4().as_bytes().to_vec();

        let mut tx = Self {
            id: vec![],
            vin: vec![txinput],
            vout: vec![txout],
        };

        tx.id = tx.hash();
        tx
    }

    pub fn get_id(&self) -> &[u8] {
        self.id.as_slice()
    }

    pub fn get_id_bytes(&self) -> Vec<u8> {
        self.id.clone()
    }

    pub fn get_vin(&self) -> &[TxInput] {
        self.vin.as_slice()
    }

    pub fn get_vout(&self) -> &[TxOutput] {
        self.vout.as_slice()
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap().to_vec()
    }

    pub fn deserialize(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }

    pub fn utxo_transaction(from: &str, to: &str, amount: i32, utxo_set: &UTXOSet) -> Transaction {
        let wallets = Wallets::new();
        let wallet = wallets.get_wallet(from).expect("unable to found wallet");
        let pub_key_hash = wallets::hash_pub_key(wallet.get_pub_key());

        let (accumulated_amount, valid_outputs) =
            utxo_set.find_spendable_outputs(pub_key_hash.as_slice(), amount);

        if accumulated_amount < amount {
            panic!("Error: Not enough balance.");
        }

        let mut inputs = vec![];
        for (txid_hex, outs) in valid_outputs {
            let txid = HEXLOWER.decode(txid_hex.as_bytes()).unwrap();
            for out in outs {
                let input = TxInput {
                    txid: txid.clone(),
                    vout: out,
                    signature: vec![],
                    pub_key: wallet.get_pub_key().to_vec(),
                };

                inputs.push(input);
            }
        }
        let mut outputs = vec![TxOutput::new(amount, to)];
        if accumulated_amount > amount {
            outputs.push(TxOutput::new(accumulated_amount - amount, from));
        }

        let mut tx = Transaction {
            id: vec![],
            vin: inputs,
            vout: outputs,
        };

        tx.id = tx.hash();

        tx.sign(utxo_set.get_blockchain(), wallet.get_pkcs8());
        tx
    }

    fn trimmed_copy(&self) -> Transaction {
        let mut inputs = vec![];
        let mut outputs = vec![];
        for input in &self.vin {
            let txinput = TxInput::new(input.get_txid(), input.get_vout());
            inputs.push(txinput);
        }

        for output in &self.vout {
            outputs.push(output.clone());
        }

        Transaction {
            id: self.id.clone(),
            vin: inputs,
            vout: outputs,
        }
    }

    fn sign(&mut self, blockchain: &Blockchain, pkcs8: &[u8]) {
        let mut tx_copy = self.trimmed_copy();

        for (idx, vin) in self.vin.iter_mut().enumerate() {
            let prev_tx_option = blockchain.find_transaction(vin.get_txid());
            if prev_tx_option.is_none() {
                panic!("Error: Previous tx is not correct");
            }

            let prev_tx = prev_tx_option.unwrap();
            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let signature = ecdsa_p256_sha256_sign_digest(pkcs8, tx_copy.get_id());
            vin.signature = signature;
        }
    }

    pub fn verify(&self, blockchain: &Blockchain) -> bool {
        if self.is_coinbase() {
            return true;
        }

        let mut tx_copy = self.trimmed_copy();
        for (idx, vin) in self.vin.iter().enumerate() {
            let prev_tx_option = blockchain.find_transaction(vin.get_txid());

            if prev_tx_option.is_none() {
                panic!("Error: previous tx is not correct");
            }

            let prev_tx = prev_tx_option.unwrap();
            tx_copy.vin[idx].signature = vec![];
            tx_copy.vin[idx].pub_key = prev_tx.vout[vin.vout].pub_key_hash.clone();
            tx_copy.id = tx_copy.hash();
            tx_copy.vin[idx].pub_key = vec![];

            let verify = ecdsa_p256_sha256_sign_verify(
                vin.pub_key.as_slice(),
                vin.signature.as_slice(),
                tx_copy.get_id(),
            );

            if !verify {
                return false;
            }
        }
        true
    }

    pub fn is_coinbase(&self) -> bool {
        return self.vin.len() == 1 && self.vin[0].pub_key.len() == 0;
    }

    fn hash(&self) -> Vec<u8> {
        let tx_copy = Transaction {
            id: vec![],
            vin: self.vin.clone(),
            vout: self.vout.clone(),
        };

        sha256_digest(tx_copy.serialize().as_slice())
    }
}
