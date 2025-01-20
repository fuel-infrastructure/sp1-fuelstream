
Forked from [sp1-blobstream using commit cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9, supporter sp1 v3.0.0](https://github.com/succinctlabs/sp1-blobstream/tree/cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9)

## Build & Run

Circuit Building to Create Elf:

```sh
cd ./program
cargo prove build --docker --tag v3.0.0 --elf-name fuelstreamx-elf
```

Running: 

```sh
cargo run --release --bin operator
```

To get the genesis parameters for the smart-contract:

```sh
cargo run --release --bin genesis
```

## Tests

```sh
cd ./scripts
cargo test
```
