//! To run the binary:
//!
//!     `cargo run --release --bin genesis -- --block <height>`
use alloy::primitives::hex::encode_prefixed;
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

    let mut tendermint_client = FuelStreamXTendermintClient::new(tendermint_rpc_url).await;

    // -------- SP1 Config

    let plonk_client = FuelStreamXPlonkClient::new().await;

    // -------- Run

    let block = tendermint_client.fetch_light_block(args.block);
    info!(
        "\nGENESIS_HEIGHT={:?}\nGENESIS_HEADER={}\nVKEY={}\n",
        block.height().value(),
        encode_prefixed(block.signed_header.header.hash()),
        plonk_client.get_vkey_hash()
    );
}
