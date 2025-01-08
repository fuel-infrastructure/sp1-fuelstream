//! To run the binary:
//!
//!     `cargo run --release --bin operator`
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{Address, B256},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    signers::local::PrivateKeySigner,
    sol,
    transports::http::{Client, Http},
};
use anyhow::Result;
use core::str::FromStr;
use fuelstreamx_sp1_script::ethereum_client::FuelStreamXEthereumClient;
use fuelstreamx_sp1_script::plonk_client::FuelStreamXPlonkClient;
use fuelstreamx_sp1_script::tendermint_client::FuelStreamXTendermintClient;
use log::{error, info};
use primitives::get_header_update_verdict;
use primitives::types::ProofInputs;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tendermint_light_client_verifier::Verdict;
use tendermint_rpc::Url;

// Timeout for the proof in seconds.
const PROOF_TIMEOUT_SECONDS: u64 = 60 * 30;

const NUM_RELAY_RETRIES: u32 = 3;

// impl SP1BlobstreamOperator {

//     /// Check the verifying key in the contract matches the verifying key in the prover.
//     async fn check_vkey(&self) -> Result<()> {
//         let contract = SP1Blobstream::new(self.contract_address, self.wallet_filler.clone());
//         let verifying_key = contract
//             .blobstreamProgramVkey()
//             .call()
//             .await?
//             .blobstreamProgramVkey;

//         if verifying_key.0.to_vec()
//             != hex::decode(self.vk.bytes32().strip_prefix("0x").unwrap()).unwrap()
//         {
//             return Err(anyhow::anyhow!(
//                     "The verifying key in the operator does not match the verifying key in the contract!"
//                 ));
//         }

//         Ok(())
//     }

//     async fn request_header_range(
//         &self,
//         trusted_block: u64,
//         target_block: u64,
//     ) -> Result<SP1ProofWithPublicValues> {
//         let prover = TendermintProver::new();
//         let mut stdin = SP1Stdin::new();

//         let inputs = prover
//             .fetch_input_for_blobstream_proof(trusted_block, target_block)
//             .await;

//         // Simulate the step from the trusted block to the target block.
//         let verdict =
//             get_header_update_verdict(&inputs.trusted_light_block, &inputs.target_light_block);
//         assert_eq!(verdict, Verdict::Success);

//         let encoded_proof_inputs = serde_cbor::to_vec(&inputs)?;
//         stdin.write_vec(encoded_proof_inputs);

//         self.client
//             .prove(&self.pk, stdin)
//             .plonk()
//             .timeout(Duration::from_secs(PROOF_TIMEOUT_SECONDS))
//             .run()
//     }

//     /// Relay a header range proof to the SP1 Blobstream contract.
//     async fn relay_header_range(&self, proof: SP1ProofWithPublicValues) -> Result<B256> {
//         // TODO: sp1_sdk should return empty bytes in mock mode.
//         let proof_as_bytes = if env::var("SP1_PROVER").unwrap().to_lowercase() == "mock" {
//             vec![]
//         } else {
//             proof.bytes()
//         };

//         let contract = SP1Blobstream::new(self.contract_address, self.wallet_filler.clone());

//         if self.use_kms_relayer {
//             let proof_bytes = proof_as_bytes.clone().into();
//             let public_values = proof.public_values.to_vec().into();
//             let commit_header_range = contract.commitHeaderRange(proof_bytes, public_values);
//             relay::relay_with_kms(
//                 &relay::KMSRelayRequest {
//                     chain_id: self.chain_id,
//                     address: self.contract_address.to_checksum(None),
//                     calldata: commit_header_range.calldata().to_string(),
//                     platform_request: false,
//                 },
//                 NUM_RELAY_RETRIES,
//             )
//             .await
//         } else {
//             let public_values_bytes = proof.public_values.to_vec();

//             let gas_limit = relay::get_gas_limit(self.chain_id);
//             let max_fee_per_gas =
//                 relay::get_fee_cap(self.chain_id, self.wallet_filler.root()).await;

//             let nonce = self
//                 .wallet_filler
//                 .get_transaction_count(self.relayer_address)
//                 .await?;

//             // Wait for 3 required confirmations with a timeout of 60 seconds.
//             const NUM_CONFIRMATIONS: u64 = 3;
//             const TIMEOUT_SECONDS: u64 = 60;
//             let receipt = contract
//                 .commitHeaderRange(proof_as_bytes.into(), public_values_bytes.into())
//                 .gas_price(max_fee_per_gas)
//                 .gas(gas_limit)
//                 .nonce(nonce)
//                 .send()
//                 .await?
//                 .with_required_confirmations(NUM_CONFIRMATIONS)
//                 .with_timeout(Some(Duration::from_secs(TIMEOUT_SECONDS)))
//                 .get_receipt()
//                 .await?;

//             // If status is false, it reverted.
//             if !receipt.status() {
//                 error!("Transaction reverted!");
//             }

//             Ok(receipt.transaction_hash)
//         }
//     }

//     async fn run(&self) -> Result<()> {
//         self.check_vkey().await?;

//         let fetcher = TendermintRPCClient::default();
//         let block_update_interval = get_block_update_interval();

//         let contract = SP1Blobstream::new(self.contract_address, self.wallet_filler.clone());

//         // Read the data commitment max from the contract.
//         let data_commitment_max = contract
//             .DATA_COMMITMENT_MAX()
//             .call()
//             .await?
//             .DATA_COMMITMENT_MAX;

//         // Get the latest block from the contract.
//         let current_block = contract.latestBlock().call().await?.latestBlock;

//         // Get the head of the chain.
//         let latest_tendermint_block_nb = fetcher.get_latest_block_height().await;

//         // Subtract 1 block to ensure the block is stable.
//         let latest_stable_tendermint_block = latest_tendermint_block_nb - 1;

//         // block_to_request is the closest interval of block_interval less than min(latest_stable_tendermint_block, data_commitment_max + current_block)
//         let max_block = std::cmp::min(
//             latest_stable_tendermint_block,
//             data_commitment_max + current_block,
//         );
//         let block_to_request = max_block - (max_block % block_update_interval);

//         // If block_to_request is greater than the current block in the contract, attempt to request.
//         if block_to_request > current_block {
//             // The next block the operator should request.
//             let max_end_block = block_to_request;

//             let target_block = fetcher
//                 .find_block_to_request(current_block, max_end_block)
//                 .await;

//             info!("Current block: {}", current_block);
//             info!("Attempting to step to block {}", target_block);

//             // Request a header range if the target block is not the next block.
//             match self.request_header_range(current_block, target_block).await {
//                 Ok(proof) => {
//                     let tx_hash = self.relay_header_range(proof).await?;
//                     info!(
//                         "Posted data commitment from block {} to block {}\nTransaction hash: {}",
//                         current_block, target_block, tx_hash
//                     );
//                 }
//                 Err(e) => {
//                     return Err(anyhow::anyhow!("Header range request failed: {}", e));
//                 }
//             };
//         } else {
//             info!("Next block to request is {} which is > the head of the Tendermint chain which is {}. Sleeping.", block_to_request + block_update_interval, latest_stable_tendermint_block);
//         }
//         Ok(())
//     }
// }

// fn get_loop_interval_mins() -> u64 {
//     let loop_interval_mins_env = env::var("LOOP_INTERVAL_MINS");
//     let mut loop_interval_mins = 60;
//     if loop_interval_mins_env.is_ok() {
//         loop_interval_mins = loop_interval_mins_env
//             .unwrap()
//             .parse::<u64>()
//             .expect("invalid LOOP_INTERVAL_MINS");
//     }
//     loop_interval_mins
// }

// fn get_block_update_interval() -> u64 {
//     let block_update_interval_env = env::var("BLOCK_UPDATE_INTERVAL");
//     let mut block_update_interval = 360;
//     if block_update_interval_env.is_ok() {
//         block_update_interval = block_update_interval_env
//             .unwrap()
//             .parse::<u64>()
//             .expect("invalid BLOCK_UPDATE_INTERVAL");
//     }
//     block_update_interval
// }

struct FuelStreamXOperator {
    ethereum_client: FuelStreamXEthereumClient,
    tendermint_client: FuelStreamXTendermintClient,
    plonk_client: FuelStreamXPlonkClient,
}

impl FuelStreamXOperator {
    /// Constructs a new FuelStreamX operator. Expects that the .env file is loaded
    pub async fn new() -> Self {
        // -------- Ethereum Config

        let ethereum_rpc_url = env::var("RPC_URL").expect("RPC_URL not set");
        let private_key = env::var("PRIVATE_KEY").expect("PRIVATE_KEY not set");
        let contract_address = env::var("CONTRACT_ADDRESS").expect("CONTRACT_ADDRESS not set");

        let ethereum_client = FuelStreamXEthereumClient::new(
            ethereum_rpc_url.as_str(),
            private_key.as_str(),
            contract_address.as_str(),
        )
        .await;

        // -------- Tendermint Config

        let tendermint_rpc_url_env =
            env::var("TENDERMINT_RPC_URL").expect("TENDERMINT_RPC_URL not set");
        let tendermint_rpc_url = Url::from_str(&tendermint_rpc_url_env)
            .expect("failed to parse TENDERMINT_RPC_URL string");

        let tendermint_client = FuelStreamXTendermintClient::new(tendermint_rpc_url).await;

        // -------- SP1 Config

        let plonk_client = FuelStreamXPlonkClient::new().await;

        Self {
            ethereum_client,
            tendermint_client,
            plonk_client,
        }
    }

    /// Check the verifying key in the contract matches the verifying key in the prover.
    async fn check_vkey(&self) -> Result<()> {
        let ethereum_v_key = self.ethereum_client.get_v_key().await;
        let prover_v_key = self.plonk_client.get_vkey_hash();

        if ethereum_v_key.to_string() != prover_v_key {
            return Err(anyhow::anyhow!(
                "the verifying key of the elf does not match the verifying key in the contract"
            ));
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let operator = FuelStreamXOperator::new().await;
    operator.check_vkey().await.expect("vKey does not match")
}

// #[tokio::main]
// async fn main() {
//     dotenv::dotenv().ok();

//     // -------- Ethereum Config

//     let operator = SP1BlobstreamOperator::new().await;

//     info!("Starting SP1 Blobstream operator");
//     const LOOP_TIMEOUT_MINS: u64 = 20;
//     loop {
//         let request_interval_mins = get_loop_interval_mins();
//         // If the operator takes longer than LOOP_TIMEOUT_MINS for a single invocation, or there's
//         // an error, sleep for the loop interval and try again.
//         if let Err(e) = tokio::time::timeout(
//             tokio::time::Duration::from_secs(60 * LOOP_TIMEOUT_MINS),
//             operator.run(),
//         )
//         .await
//         {
//             error!("Error running operator: {}", e);
//         }
//         tokio::time::sleep(tokio::time::Duration::from_secs(60 * request_interval_mins)).await;
//     }
// }
