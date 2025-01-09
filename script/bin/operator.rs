//! To run the binary:
//!
//!     `cargo run --release --bin operator`
use alloy::primitives::B256;
use anyhow::{Ok, Result};
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
use tendermint::node::info;
use tendermint_light_client::verifier::types::Height;
use tendermint_light_client_verifier::Verdict;
use tendermint_rpc::{Client, Url};

// Timeout for the proof in seconds.
const PROOF_TIMEOUT_SECONDS: u64 = 60 * 30;

const NUM_RELAY_RETRIES: u32 = 3;

// impl SP1BlobstreamOperator {

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
//         Ok(())
//     }
// }

struct FuelStreamXOperator {
    ethereum_client: FuelStreamXEthereumClient,
    tendermint_client: FuelStreamXTendermintClient,
    plonk_client: FuelStreamXPlonkClient,
    minimum_block_range: u64,
}

impl FuelStreamXOperator {
    /// Constructs a new FuelStreamX operator. Expects that the .env file is loaded
    pub async fn new() -> Self {
        let minimum_block_range = env::var("MINIMUM_BLOCK_RANGE")
            .map(|t| {
                t.parse::<u64>()
                    .expect("MINIMUM_BLOCK_RANGE must be a valid number")
            })
            .unwrap_or(512);

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

        let sp1_timeout = env::var("SP1_TIMEOUT_MINS")
            .map(|t| {
                t.parse::<u64>()
                    .expect("SP1_TIMEOUT_MINS must be a valid number")
            })
            .unwrap_or(60);

        let plonk_client = FuelStreamXPlonkClient::new(sp1_timeout * 60).await;

        Self {
            ethereum_client,
            tendermint_client,
            plonk_client,
            minimum_block_range,
        }
    }

    /// Check the verifying key in the contract matches the verifying key in the prover.
    async fn check_v_key(&self) -> Result<()> {
        let ethereum_v_key = self.ethereum_client.get_v_key().await;
        let prover_v_key = self.plonk_client.get_v_key_hash();

        if ethereum_v_key.to_string() != prover_v_key {
            return Err(anyhow::anyhow!(
                "the verifying key of the elf does not match the verifying key in the contract"
            ));
        }

        Ok(())
    }

    async fn run(&self) -> Result<()> {
        self.check_v_key().await?;

        // Get latest light client sync from Ethereum
        let bridge_commitment_max = self.ethereum_client.get_bridge_commitment_max().await;
        let (light_client_height, light_client_hash) = self.ethereum_client.get_latest_sync().await;

        // Assertion to check if a correct tendermint node is in use
        assert!(light_client_hash == B256::from_slice(
            self.tendermint_client
                .rpc_client
                .commit(Height::try_from(light_client_height).unwrap())
                .await
                .expect("failed to get a commit from the Tendermint node")
                .signed_header
                .header
                .hash()
                .as_bytes()
        ),
            "latest light client header hash on Ethereum does not match with the corresponding one on the Tendermint node"
        );

        // Get the head of the Tendermint chain
        let latest_tendermint_block = self
            .tendermint_client
            .rpc_client
            .latest_commit()
            .await
            .expect("failed to get the latest commit from Tendermint node");

        // Maximum light client iteration
        let max_block = std::cmp::min(
            latest_tendermint_block.signed_header.header.height.value(),
            light_client_height + bridge_commitment_max,
        );

        if max_block == light_client_height
            || (max_block - light_client_height) < self.minimum_block_range
        {
            info!("not enough blocks have been generated for a new light client update, sleeping");
            return Ok(());
        }

        // Get input for proof
        let proof_inputs = self
            .tendermint_client
            .fetch_proof_inputs(light_client_height, max_block);

        //         let encoded_proof_inputs = serde_cbor::to_vec(&inputs)?;
        //         stdin.write_vec(encoded_proof_inputs);

        //         self.client
        //             .prove(&self.pk, stdin)
        //             .plonk()
        //             .timeout(Duration::from_secs(PROOF_TIMEOUT_SECONDS))
        //             .run()

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    // Run operator with a timeout, the timeout should be long enough to generate a ZK proof
    let operator = FuelStreamXOperator::new().await;
    if let Err(e) = tokio::time::timeout(
        tokio::time::Duration::from_secs(60 * operator.plonk_client.timeout),
        operator.run(),
    )
    .await
    {
        error!("Error running operator: {}", e);
    }
}
