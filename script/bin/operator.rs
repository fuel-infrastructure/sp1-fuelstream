//! To run the binary:
//!
//!     `cargo run --release --bin operator`
use alloy::primitives::B256;
use anyhow::Result;
use core::str::FromStr;
use fuelstreamx_sp1_script::ethereum_client::FuelStreamXEthereumClient;
use fuelstreamx_sp1_script::plonk_client::FuelStreamXPlonkClient;
use fuelstreamx_sp1_script::tendermint_client::FuelStreamXTendermintClient;
use log::{error, info};
use std::env;
use std::result::Result::Ok;
use tendermint_light_client::verifier::types::Height;
use tendermint_rpc::{Client, Url};

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

    async fn run(&mut self) -> Result<()> {
        self.check_v_key().await?;

        // Get latest light client sync from Ethereum
        // let bridge_commitment_max = self.ethereum_client.get_bridge_commitment_max().await;
        let bridge_commitment_max = 5;
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

        // Call the circuit
        let proof_inputs = self
            .tendermint_client
            .fetch_proof_inputs(light_client_height, max_block)
            .await;

        match self.plonk_client.generate_proof(proof_inputs).await {
            Ok(proof_output) => {
                println!("{:?}", proof_output);
                info!("successfully generated proof")
            }
            Err(e) => {
                return Err(anyhow::anyhow!("failed to generate proof: {}", e));
            }
        };

        // TODO: Submit proof on-chain

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    // Run operator with a timeout, the timeout should be long enough to generate a ZK proof
    let mut operator = FuelStreamXOperator::new().await;
    if let Err(e) = tokio::time::timeout(
        tokio::time::Duration::from_secs(operator.plonk_client.timeout),
        operator.run(),
    )
    .await
    {
        error!("Error running operator: {}", e);
    }
}
