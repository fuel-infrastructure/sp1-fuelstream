//! To run the binary:
//!
//!     `cargo run --release --bin genesis -- --block <height>`
use alloy::primitives::B256;
use clap::Parser;
use core::str::FromStr;
use fuelstreamx_sp1_script::plonk_client::FuelStreamXPlonkClient;
use fuelstreamx_sp1_script::tendermint_client::FuelStreamXTendermintClient;
use log::info;
use std::env;
use tendermint_rpc::Url;

#[derive(Parser, Debug, Clone)]
#[command(about = "Get the genesis parameters from a given block.")]
pub struct GenesisArgs {
    #[arg(long)]
    pub block: u64,
}

#[tokio::main]
pub async fn main() {
    env::set_var("RUST_LOG", "info");

    dotenv::dotenv().ok();
    env_logger::init();
    let args = GenesisArgs::parse();

    // -------- Tendermint Config

    let tendermint_rpc_url_env =
        env::var("TENDERMINT_RPC_URL").expect("TENDERMINT_RPC_URL not set");
    let tendermint_rpc_url =
        Url::from_str(&tendermint_rpc_url_env).expect("failed to parse TENDERMINT_RPC_URL string");
    let tendermint_grpc_url_env =
        env::var("TENDERMINT_GRPC_URL").expect("TENDERMINT_GRPC_URL not set");

    let mut tendermint_client =
        FuelStreamXTendermintClient::new(tendermint_rpc_url, tendermint_grpc_url_env).await;

    // -------- SP1 Config

    let plonk_client = FuelStreamXPlonkClient::new(0).await;

    // -------- Run

    let block = tendermint_client.fetch_light_block(args.block);
    info!(
        "\nGENESIS_HEIGHT={:?}\nGENESIS_HEADER={}\nVKEY={}\n",
        block.height().value(),
        B256::from_slice(block.signed_header.header.hash().as_bytes()),
        plonk_client.get_v_key_hash()
    );
}
