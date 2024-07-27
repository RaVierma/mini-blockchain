# Mini Blockchain (In Rust)
This is mini blockchain written in Rust in programming language. You can use it to understand the working of blockchain.

## Features

- **create blockchain**: You can create blockchain.
- **create wallet**: You can create wallet.
- **getbalance**: Check the balance of address.
- **list addresses**: List the addresses of wallet.
- **print chain**: Print all block in blockchain.
- **reindex utxo**: Reindex the UTXO index.
- **send transaction**: Do transaction.
- **start node**: Start a node for mining.

## Installation and build

Before installing the Mini Blockchain, ensure you have the following prerequisites:

1. **Rust**: Make sure you have Rust installed and version must be 1.79 or higher. You can install it via [rustup](https://www.rust-lang.org/tools/install).

Now you're ready to install the  mini-blockchain. Clone this repository and build the application using Cargo:
```bash
    git clone https://github.com/your_username/mini-blockchain.git
    cd mini-blockchain
    cargo build --release
```

## Usage

- Go to release Directory
```bash
    cd mini-blockchain/target/release
```

- Check for avialble subcommands
```bash
    ./mini-blockchain -h
```

- Create wallet
```bash
    ./mini-blockchain createwallet
```

- Create blockchain
```bash
    ./mini-blockchain createblockchain --address 1CsgUd1p764vKKS3bVE1xrULPAUpkhawb3
```

- List addresses
```bash
    ./mini-blockchain listaddresses
```

- Send transaction
```bash
    ./mini-blockchain send --from 1CsgUd1p764vKKS3bVE1xrULPAUpkhawb4 --to 1CsgUd1p764vKKS3bVE1xrULPAUpkhawb3 --amount 5 --mine 1
```

- Start node
```bash
    ./mini-blockchain startnode --miner 1CsgUd1p764vKKS3bVE1xrULPAUpkhawb3
```

- Get balance of a address
```bash
    ./mini-blockchain getbalance --address 1CsgUd1p764vKKS3bVE1xrULPAUpkhawb3
```

- Print all block of blockchain 
```bash
    ./mini-blockchain printchain
```

## Disclaimer
**Not Production Use**. This blockcahin build for learning purpose only.
