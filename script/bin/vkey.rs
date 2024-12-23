//! To build the binary:
//!
//!     `cargo build --release --bin vkey`
//!
//!
//!
//!
//!
//!
use sp1_sdk::{HashableKey, ProverClient};
const FUELSTREAMX_ELF: &[u8] = include_bytes!("../../elf/fuelstreamx-elf");

#[tokio::main]
pub async fn main() {
    let client = ProverClient::new();
    let (_pk, vk) = client.setup(FUELSTREAMX_ELF);
    println!("fuelstreamx-elf VK: {}", vk.bytes32());
}
