//! To build the binary:
//!
//!     `cargo build --release --bin vkey`
use primitives::types::FUELSTREAMX_ELF;
use sp1_sdk::{HashableKey, ProverClient};

#[tokio::main]
pub async fn main() {
    let client = ProverClient::new();
    let (_pk, vk) = client.setup(FUELSTREAMX_ELF);
    println!("fuelstreamx-elf VK: {}", vk.bytes32());
}
