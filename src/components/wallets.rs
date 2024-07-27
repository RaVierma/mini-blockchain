use std::{
    collections::HashMap,
    env::current_dir,
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Write},
};

use ring::signature::{EcdsaKeyPair, KeyPair, ECDSA_P256_SHA256_FIXED_SIGNING};
use serde::{Deserialize, Serialize};

use super::helpers::{
    base58_decode, base58_encode, create_key_pair, ripemd160_digest, sha256_digest,
};

const VERSION: u8 = 0x00;

pub const ADDRESS_CHECK_SUM_LEN: usize = 4;

pub const WALLET_FILE: &str = "wallet.dat";

#[derive(Clone, Serialize, Deserialize)]
pub struct Wallet {
    pkcs8: Vec<u8>,
    pub_key: Vec<u8>,
}

impl Wallet {
    pub fn new() -> Self {
        let pkcs8 = create_key_pair();
        let rng = ring::rand::SystemRandom::new();
        let key_pair =
            EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &pkcs8, &rng).unwrap();
        let pub_key = key_pair.public_key().as_ref().to_vec();
        Self { pkcs8, pub_key }
    }

    pub fn get_address(&self) -> String {
        let pub_key_hash = hash_pub_key(self.pub_key.as_slice());
        let mut payload: Vec<u8> = vec![];
        payload.push(VERSION);
        payload.extend(pub_key_hash.as_slice());
        let chekshum = checksum(payload.as_slice());
        payload.extend(chekshum.as_slice());
        base58_encode(payload.as_slice())
    }

    pub fn get_pub_key(&self) -> &[u8] {
        self.pub_key.as_slice()
    }

    pub fn get_pkcs8(&self) -> &[u8] {
        self.pkcs8.as_slice()
    }
}

pub fn hash_pub_key(pub_key: &[u8]) -> Vec<u8> {
    let pub_key_sha256 = sha256_digest(pub_key);
    ripemd160_digest(pub_key_sha256.as_slice())
}

fn checksum(payload: &[u8]) -> Vec<u8> {
    let first_sha = sha256_digest(payload);
    let second_sh = sha256_digest(first_sha.as_slice());
    second_sh[0..ADDRESS_CHECK_SUM_LEN].to_vec()
}

pub fn validate_address(address: &str) -> bool {
    let payload = base58_decode(address);
    let actual_checksum = payload[payload.len() - ADDRESS_CHECK_SUM_LEN..].to_vec();
    let version = payload[0];
    let pub_key_hash = payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN].to_vec();

    let mut target_vec = vec![];
    target_vec.push(version);
    target_vec.extend(pub_key_hash);
    let target_checksum = checksum(target_vec.as_slice());
    actual_checksum.eq(target_checksum.as_slice())
}

pub fn convert_address(pub_key_hash: &[u8]) -> String {
    let mut payload: Vec<u8> = vec![];
    payload.push(VERSION);
    payload.extend(pub_key_hash);
    let checksum = checksum(payload.as_slice());
    payload.extend(checksum.as_slice());
    base58_encode(payload.as_slice())
}

// wallets

pub struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn new() -> Self {
        let mut wallets = Wallets {
            wallets: HashMap::new(),
        };

        wallets.load_from_file();
        wallets
    }

    pub fn create_wallets(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_address();
        self.wallets.insert(address.clone(), wallet);
        self.save_to_file();
        address
    }

    pub fn get_addresses(&self) -> Vec<String> {
        let mut addresses = vec![];

        for (address, _) in &self.wallets {
            addresses.push(address.clone());
        }
        addresses
    }

    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        if let Some(wallet) = self.wallets.get(address) {
            return Some(wallet);
        }
        None
    }

    pub fn load_from_file(&mut self) {
        let path = current_dir().unwrap().join(WALLET_FILE);
        if !path.exists() {
            return;
        }

        let mut file = File::open(path).unwrap();
        let metadata = file.metadata().expect("unable to read metadata");
        let mut buf = vec![0; metadata.len() as usize];
        let _ = file.read(&mut buf).expect("buffer overflow");
        let wallets = bincode::deserialize(&buf[..]).expect("unable to deserialize the file data");
        self.wallets = wallets;
    }

    fn save_to_file(&self) {
        let path = current_dir().unwrap().join(WALLET_FILE);
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&path)
            .expect("unable to open wallet.dat");

        let mut writer = BufWriter::new(file);
        let wallet_bytes =
            bincode::serialize(&self.wallets).expect("unable to serialize the wallets");
        writer.write(wallet_bytes.as_slice()).unwrap();
        let _ = writer.flush();
    }
}
