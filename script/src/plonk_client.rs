use primitives::types::FUELSTREAMX_ELF;
use sp1_sdk::{HashableKey, ProverClient, SP1ProvingKey, SP1VerifyingKey};

pub struct FuelStreamXPlonkClient {
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
    pub timeout: u64,
}

impl FuelStreamXPlonkClient {
    /// Constructs a new FuelStreamX plonk client
    pub async fn new(timeout: u64) -> Self {
        let prover_client = ProverClient::new();
        let (pk, vk) = prover_client.setup(FUELSTREAMX_ELF);

        Self { pk, vk, timeout }
    }

    pub fn get_v_key_hash(&self) -> String {
        self.vk.bytes32()
    }
}
