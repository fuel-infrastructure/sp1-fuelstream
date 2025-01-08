use primitives::types::FUELSTREAMX_ELF;
use sp1_sdk::{HashableKey, ProverClient, SP1ProvingKey, SP1VerifyingKey};

pub struct FuelStreamXPlonkClient {
    pk: SP1ProvingKey,
    vk: SP1VerifyingKey,
}

impl FuelStreamXPlonkClient {
    /// Constructs a new FuelStreamX plonk client
    pub async fn new() -> Self {
        let prover_client = ProverClient::new();
        let (pk, vk) = prover_client.setup(FUELSTREAMX_ELF);

        Self { pk, vk }
    }

    pub fn get_vkey_hash(&self) -> String {
        self.vk.bytes32()
    }
}
