//! To run the binary:
//!
//!     `cargo run --release --bin vkey`
use fuelstreamx_sp1_script::plonk_client::FuelStreamXPlonkClient;
use log::info;
use std::env;

#[tokio::main]
pub async fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let plonk_client = FuelStreamXPlonkClient::new(0).await;
    info!("VKEY={}\n", plonk_client.get_v_key_hash());
}
