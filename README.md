# SP1-FuelStreamX

SP1-FuelStreamX is an implementation of a [fuel-sequencer light client](https://github.com/fuel-infrastructure/fuel-sequencer) on Ethereum. 
The role of this repository is to generate a `plonk` zk proof that is verifiable on Ethereum which allows 
the light client to safely validate non-trusted blocks and [bridge commitment submissions](https://github.com/FuelLabs/fuel-rollup/blob/654dfb843fc66e09801ec0e399fd4e95203a1d15/contracts/fuelstreamx/FuelStreamX.sol#L120). 
These commitments allow users to withdraw from the fuel-sequencer to Ethereum and manage the accounting of the `fuel` token. 
The components are: 

`/program`: The SP1 FuelStreamX circuit that verifies the next light client step.
`/primitives`: Libraries for types and helper functions used in the program.
`/script`: Scripts for getting the contract's genesis parameters and deploying the operator to update the light client.

Forked from [sp1-blobstream using commit cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9](https://github.com/succinctlabs/sp1-blobstream/tree/cbd1ee173a9acf8cda80bf4b6ed093623dd7e0a9), currently supporting [sp1 v3.0.0](https://github.com/succinctlabs/sp1/releases/tag/v3.0.0).

## Build & Run with Docker

This step is only required if there were changes to the circuit found in `/program`. 
You first need to download [sp1](https://docs.succinct.xyz/docs/getting-started/install). Afterwards, build the circuit to generate the `elf`:

```sh
cd ./program && cargo prove build --docker --tag v3.0.0 --elf-name fuelstreamx-elf
```

To build operator: 

```sh
docker build -t sp1-fuelstreamx-operator  .
```

To run the operator use the following command. This allows the container to be restarted every `x` 
minutes regardless of the exit code:

```sh
docker run --restart=always --env-file .env sp1-fuelstreamx-operator
```

## Tests

```sh
cargo test -p sp1-fuelstreamx-script
```

## Ethereum Contract Parameters

To get the genesis parameters for the smart-contract:

```sh
cargo run --release --bin genesis -- --block <height>
```

This gives the `vKey`, the `genesis height` and the `genesis header hash`. 