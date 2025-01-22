# FuelStreamX

Forked from [sp1-blobstream using commit cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9, supporter sp1 v3.0.0](https://github.com/succinctlabs/sp1-blobstream/tree/cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9)

## Build & Run with Docker

Circuit Building to Create Elf:

```sh
cd ./program
cargo prove build --docker --tag v3.0.0 --elf-name fuelstreamx-elf
```

To build operator: 

```sh
docker build -t sp1-fuelstreamx-operator  .
```

To run the operator use the following command. This always the container to be restarted every `x` 
minutes regardless of the exit code:

```
docker run --restart=always --env-file .env sp1-fuelstreamx-operator
```

## Tests

```sh
cd ./scripts
cargo test
```

## Ethereum Contract Parameters

To get the genesis parameters for the smart-contract:

```sh
cargo run --release --bin genesis -- --block <height>
```

This gives the `vKey`, the `genesis height` and the `genesis header hash`. 