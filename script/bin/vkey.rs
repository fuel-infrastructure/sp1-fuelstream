use sp1_sdk::{HashableKey, ProverClient};
const BLOBSTREAMX_ELF: &[u8] = include_bytes!("../../elf/fuelstreamx-elf");

#[tokio::main]
pub async fn main() {
    let client = ProverClient::new();
    let (_pk, vk) = client.setup(BLOBSTREAMX_ELF);
    println!("VK: {}", vk.bytes32());
}
