use components::{blockchain::Blockchain, config::GLOBAL_CONFIG, helpers::base58_decode, server::{send_tx, Server, CENERAL_NODE}, transaction::Transaction, utxoset::UTXOSet, wallets::{convert_address, hash_pub_key, validate_address, Wallet, Wallets, ADDRESS_CHECK_SUM_LEN}};
use data_encoding::HEXLOWER;
use structopt::StructOpt;

mod components;



const MINE_TRUE: usize = 1;

#[derive(Debug, StructOpt)]
#[structopt(name = "mini_blockchain")]
struct Opt {
    #[structopt(subcommand)]
    command: Command
}

#[derive(StructOpt, Debug)]
enum Command {
    #[structopt(name = "createblockchain", about="Create a new blockchain")]
    Createblockchain {
        #[structopt(short, long,name = "address", help="The address to send genesis block reward to")]
        address: String,
    },
    #[structopt(name = "createwallet", about="Create a new Wallet")]
    Createwallet,
    #[structopt(name = "getbalance", about="Get the wallet of the target address")]
    GetBalance {
        #[structopt(short, long,name = "address", help="The wallet address")]
        address: String
    },
    #[structopt(name = "listaddresses", about="Print local wallet address")]
    ListAddresses,
    #[structopt(name = "send", about="Add new block to chain")]
    Send {
        #[structopt(short, long,name = "from", help="Source wallet address")]
        from: String,
        #[structopt(short, long,name = "to", help="Destination wallet address")]
        to: String,
        #[structopt(short, long,name = "amount", help="Amount to send")]
        amount: i32,
        #[structopt(short, long,name = "mine", help="Mine immediately on the same node")]
        mine: usize
    },
    #[structopt(name = "printchain", about="Print blockchain all block")]
    Printchain,
    #[structopt(name = "reindexutxo", about="rebuild UTXO index set")]
    Reindexutxo,
    #[structopt(name = "startnode", about="Start a node")]
    StartNode {
        #[structopt(short, long,name = "miner", help="Enable mining mode and send reward to ADDRESS")]
        miner: Option<String>
    }
}

fn main() {
    env_logger::builder().filter_level(log::LevelFilter::Info).init();

    let opt = Opt::from_args();

    match opt.command {
        Command::Createblockchain { address } => {
            let blockchain = Blockchain::init(&address);
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            println!("=> Blockchain created");
        },
        Command::Createwallet => {
            let mut wallet = Wallets::new();
            let address = wallet.create_wallets();
            println!("=> Your new address is: {address}");
        },
        Command::GetBalance { address } => {
            let address_valid = validate_address(&address);
            if address_valid == false {
                panic!("=> Error: Address is not valid.");
            }
            let payload = base58_decode(&address);
            let pub_key_hash = &payload[1..payload.len() - ADDRESS_CHECK_SUM_LEN];

            let blockchain = Blockchain::new();
            let utxo_set = UTXOSet::new(blockchain);
            let utxos = utxo_set.find_utxo(pub_key_hash);
            let mut balance = 0;
            for utxo in utxos {
                balance += utxo.get_value();
            }
            println!("=> Balance of {address} : {balance}");
        },
        Command::ListAddresses => {
            let wallets = Wallets::new();
            for address in wallets.get_addresses() {
                println!("=> {address}");
            }
        },
        Command::Send { from, to, amount, mine } => {
            if !validate_address(&from) {
                panic!("=> Error: Sender address is not valid");
            }

            if !validate_address(&to ) {
                panic!("=> Error: Receiver address is not valid");
            }

            let blockchain = Blockchain::new();
            let utxo_set = UTXOSet::new(blockchain.clone());

            let transaction = Transaction::utxo_transaction(&from, &to, amount, &utxo_set);

            if mine == MINE_TRUE {
                
                let coinbase_tx = Transaction::coinbase_tx(&from);
                let block = blockchain.mine_block(&vec![transaction, coinbase_tx]);

                utxo_set.update(&block);
            } else {
                send_tx(CENERAL_NODE, &transaction);
            }
            println!("=> Success");
        },
        Command::Printchain => {
            let mut block_iterator = Blockchain::new().iterator();
            loop {
                let option = block_iterator.next();
                if option.is_none() {
                    break;
                }

                let block = option.unwrap();
                println!("=> Prev block hash: {}", block.get_prev_block_hash());
                println!("=> Current block hash: {}", block.get_hash());
                println!("=> Current block timestamp: {}", block.get_timestamp());
                for tx in block.get_transactions() {
                    let cur_txid_hex = HEXLOWER.encode(tx.get_id());
                    println!("- Transaction txid_hex: {}", cur_txid_hex);

                    if tx.is_coinbase() == false {
                        for input in tx.get_vin() {
                            let txid_hex = HEXLOWER.encode(input.get_txid());
                            let pub_key_hash = hash_pub_key(input.get_pub_key());
                            let adddress = convert_address(pub_key_hash.as_slice());
                            println!("-- Input txid = {}, vout = {}, from = {}", txid_hex, input.get_vout(), adddress);
                        }
                    }

                    for output in tx.get_vout() {
                        let pub_key_hash = output.get_pub_key_hash();
                        let address = convert_address(pub_key_hash);
                        println!("-- output value = {}, to = {}", output.get_value(), address);
                    }
                    println!();
                }
            }
        },
        Command::Reindexutxo => {
            let blockchain = Blockchain::new();
            let utxo_set = UTXOSet::new(blockchain);
            utxo_set.reindex();
            let count = utxo_set.count_transaction();
            println!("=> Done! There are {} transaction in the UTXO set.", count);
        },
        Command::StartNode { miner } => {
            if let Some(addr) = miner {
                if validate_address(&addr) == false {
                    panic!("=> Wrong minner adderss!!!");
                }
                println!("=> Mining is on. Address to receive rewards: {}", addr);
                GLOBAL_CONFIG.set_mining_addr(addr);
            }

            let blockchain = Blockchain::new();
            let socket_addr = GLOBAL_CONFIG.get_node_addr();
            Server::new(blockchain).start(&socket_addr);
        },
    }
}
